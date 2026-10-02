// rustmix-wave=v0.17.0-parser-doc-repair-v2
// rustmix-wave=epub-xml-attribute-tokenizer-repair-ready
// rustmix-wave=epub-parser-stack-isolation-ready
// rustmix-wave=epub-chapter-aware-presentation-ready
// rustmix-wave=epub-watchdog-memory-pressure-repair-ready
//! Bounded reflowable EPUB reader foundation.
//!
//! The embedded target keeps EPUB processing deliberately small and explicit:
//! ZIP central-directory parsing is bounded, `META-INF/container.xml` selects
//! one OPF package, the manifest and spine are parsed without a general XML DOM,
//! XHTML is flattened into reflowable UTF-8 text, and EPUB3 navigation or EPUB2
//! NCX records become a compact table of contents. Images, CSS layout and
//! interactive links remain deferred.
//!
//! Opening a book measures every chapter once to learn its offsets in the
//! flattened text; chapter text is then re-extracted on demand, so only one
//! chapter is ever held in RAM and books of any length open.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use miniz_oxide::inflate::decompress_to_vec_with_limit;

use crate::{runtime_worker::run_named_worker, watchdog::Pacer};

/// Maximum EPUB archive bytes accepted from removable storage. Members are
/// read on demand, so this bounds offsets rather than RAM.
pub const EPUB_ARCHIVE_BYTES_LIMIT: u64 = 256 * 1024 * 1024;
/// Maximum central-directory records accepted from one EPUB.
pub const EPUB_ARCHIVE_ENTRY_LIMIT: usize = 4096;
/// Maximum compressed bytes extracted for one EPUB member.
pub const EPUB_MEMBER_COMPRESSED_LIMIT: usize = 2 * 1024 * 1024;
/// Maximum decompressed bytes extracted for one EPUB member.
pub const EPUB_MEMBER_UNCOMPRESSED_LIMIT: usize = 4 * 1024 * 1024;
/// Maximum flattened text measured for one EPUB.
pub const EPUB_TEXT_LIMIT: u64 = 64 * 1024 * 1024;
/// Maximum manifest records retained from one OPF package.
pub const EPUB_MANIFEST_LIMIT: usize = 4096;
/// Maximum spine records retained from one OPF package.
pub const EPUB_SPINE_LIMIT: usize = 2048;
/// Maximum TOC records rendered by the Reader UI.
pub const EPUB_TOC_LIMIT: usize = 2048;
/// Bumped whenever flattening changes, so saved chapter offsets are rebuilt.
pub const EPUB_TEXT_VERSION: &str = "2";
/// The end-of-central-directory record lies within this many bytes of the end.
const EOCD_SEARCH_BYTES: u64 = 65_557;
/// Elements whose content is never reading text.
const SKIPPED_ELEMENTS: [&str; 3] = ["head", "script", "style"];
/// Dedicated parser-worker stack budget. Real EPUB DEFLATE and XHTML work
/// must not run on the 16 KB firmware main task.
pub const EPUB_PARSER_WORKER_STACK_BYTES: usize = 64 * 1024;
/// Lightweight OPF-title worker stack budget. Library scans only read bounded
/// ZIP metadata and must not reserve the full parser stack for each title.
pub const EPUB_TITLE_WORKER_STACK_BYTES: usize = 32 * 1024;

/// One reflowable EPUB TOC destination. `text_offset` is an offset into the
/// book's flattened UTF-8 text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubTocEntry {
    pub label: String,
    pub text_offset: u64,
    pub spine_index: usize,
}

/// One readable spine chapter of the flattened text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubChapter {
    /// Sequential readable chapter number exposed by the Reader UI.
    pub number: usize,
    pub label: String,
    pub text_offset: u64,
    pub text_end_offset: u64,
    pub spine_index: usize,
    /// Archive member holding the chapter's XHTML.
    pub member: String,
}

/// Chapter and navigation index of one EPUB. The flattened text itself stays
/// in the archive; see [`EpubDocument::chapter_text`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubDocument {
    /// EPUB file the chapters are extracted from.
    pub path: String,
    pub title: String,
    /// OPF `dc:language` tag, such as `es` or `en-US`; empty when missing.
    pub language: String,
    pub text_size: u64,
    pub toc: Vec<EpubTocEntry>,
    pub chapters: Vec<EpubChapter>,
    pub spine_count: usize,
}

impl EpubDocument {
    #[must_use]
    pub fn text_size_bytes(&self) -> u64 {
        self.text_size
    }

    /// Resolve the readable chapter containing one flattened UTF-8 byte offset.
    #[must_use]
    pub fn chapter_for_offset(&self, offset: u64) -> Option<&EpubChapter> {
        self.chapters.iter().find(|chapter| {
            offset >= chapter.text_offset
                && (offset < chapter.text_end_offset
                    || (offset == chapter.text_end_offset
                        && chapter.text_end_offset == self.text_size_bytes()))
        })
    }

    /// Chapter shown for `offset`: gaps between chapters belong to the next
    /// chapter and offsets past the end to the last one.
    #[must_use]
    pub fn chapter_index_for_offset(&self, offset: u64) -> usize {
        self.chapters
            .iter()
            .position(|chapter| offset < chapter.text_end_offset)
            .unwrap_or(self.chapters.len().saturating_sub(1))
    }

    /// Flattened text of one chapter, extracted on a dedicated worker stack.
    pub fn chapter_text(&self, index: usize) -> Result<String, String> {
        let chapter = self
            .chapters
            .get(index)
            .ok_or_else(|| "EPUB chapter is out of range".to_string())?;
        let path = PathBuf::from(&self.path);
        let member = chapter.member.clone();
        let task = move || read_chapter_text(&path, &member);
        let text = run_named_worker("epub-chapter", EPUB_PARSER_WORKER_STACK_BYTES, task)
            .map_err(|error| error.to_string())?;
        if text.len() as u64 != chapter.text_end_offset - chapter.text_offset {
            let number = chapter.number;
            return Err(format!(
                "EPUB chapter {number} changed since it was indexed"
            ));
        }
        Ok(text)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ZipEntry {
    name: String,
    flags: u16,
    method: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_header_offset: usize,
}

#[derive(Debug)]
struct ZipArchive {
    file: File,
    size: u64,
    entries: Vec<ZipEntry>,
}

impl ZipArchive {
    /// Read the central directory; member data stays on the card until it is
    /// extracted.
    fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let file = File::open(path.as_ref()).map_err(open_error)?;
        let size = file.metadata().map_err(open_error)?.len();
        if size > EPUB_ARCHIVE_BYTES_LIMIT {
            return Err(format!("EPUB archive is too large: {size} bytes"));
        }
        let tail_size = size.min(EOCD_SEARCH_BYTES);
        let tail = read_at(&file, size - tail_size, tail_size as usize)?;
        let eocd = find_eocd(&tail).ok_or_else(|| "EPUB ZIP end record missing".to_string())?;
        let entry_count = read_u16(&tail, eocd + 10)? as usize;
        let central_size = read_u32(&tail, eocd + 12)? as usize;
        let central_offset = u64::from(read_u32(&tail, eocd + 16)?);
        if entry_count > EPUB_ARCHIVE_ENTRY_LIMIT {
            return Err(format!("EPUB ZIP has too many entries: {entry_count}"));
        }
        if central_offset + central_size as u64 > size {
            return Err("EPUB ZIP directory exceeds archive".into());
        }
        let bytes = read_at(&file, central_offset, central_size)?;
        let central_end = bytes.len();
        let mut entries = Vec::new();
        let mut cursor = 0;
        for _ in 0..entry_count {
            if read_u32(&bytes, cursor)? != 0x0201_4B50 {
                return Err("EPUB ZIP central record signature mismatch".into());
            }
            let flags = read_u16(&bytes, cursor + 8)?;
            let method = read_u16(&bytes, cursor + 10)?;
            let compressed_size = read_u32(&bytes, cursor + 20)? as usize;
            let uncompressed_size = read_u32(&bytes, cursor + 24)? as usize;
            let name_len = read_u16(&bytes, cursor + 28)? as usize;
            let extra_len = read_u16(&bytes, cursor + 30)? as usize;
            let comment_len = read_u16(&bytes, cursor + 32)? as usize;
            let local_header_offset = read_u32(&bytes, cursor + 42)? as usize;
            let name_start = cursor + 46;
            let name_end = name_start
                .checked_add(name_len)
                .ok_or_else(|| "EPUB ZIP filename overflow".to_string())?;
            if name_end > central_end {
                return Err("EPUB ZIP filename exceeds directory".into());
            }
            let name = String::from_utf8_lossy(&bytes[name_start..name_end]).replace('\\', "/");
            entries.push(ZipEntry {
                name,
                flags,
                method,
                compressed_size,
                uncompressed_size,
                local_header_offset,
            });
            cursor = name_end
                .checked_add(extra_len)
                .and_then(|value| value.checked_add(comment_len))
                .ok_or_else(|| "EPUB ZIP central record overflow".to_string())?;
            if cursor > central_end {
                return Err("EPUB ZIP central record exceeds directory".into());
            }
        }
        Ok(Self {
            file,
            size,
            entries,
        })
    }

    fn entry(&self, name: &str) -> Option<&ZipEntry> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(name))
            })
    }

    fn extract(&self, name: &str) -> Result<Vec<u8>, String> {
        let entry = self
            .entry(name)
            .ok_or_else(|| format!("EPUB member missing: {name}"))?;
        if entry.flags & 0x0001 != 0 {
            return Err(format!(
                "Encrypted EPUB member is unsupported: {}",
                entry.name
            ));
        }
        if entry.compressed_size > EPUB_MEMBER_COMPRESSED_LIMIT {
            return Err(format!(
                "EPUB member compressed size is too large: {}",
                entry.name
            ));
        }
        if entry.uncompressed_size > EPUB_MEMBER_UNCOMPRESSED_LIMIT {
            return Err(format!(
                "EPUB member expanded size is too large: {}",
                entry.name
            ));
        }
        let offset = entry.local_header_offset as u64;
        let header = read_at(&self.file, offset, 30)?;
        if read_u32(&header, 0)? != 0x0403_4B50 {
            return Err(format!("EPUB local ZIP header mismatch: {}", entry.name));
        }
        let name_len = u64::from(read_u16(&header, 26)?);
        let extra_len = u64::from(read_u16(&header, 28)?);
        let data_start = offset + 30 + name_len + extra_len;
        if data_start + entry.compressed_size as u64 > self.size {
            return Err(format!("EPUB ZIP member exceeds archive: {}", entry.name));
        }
        let compressed = read_at(&self.file, data_start, entry.compressed_size)?;
        let output = match entry.method {
            0 => compressed,
            8 => decompress_to_vec_with_limit(&compressed, EPUB_MEMBER_UNCOMPRESSED_LIMIT)
                .map_err(|error| format!("EPUB deflate failed for {}: {error:?}", entry.name))?,
            method => {
                return Err(format!(
                    "Unsupported EPUB compression method {method} for {}",
                    entry.name
                ))
            }
        };
        if output.len() > EPUB_MEMBER_UNCOMPRESSED_LIMIT {
            return Err(format!("EPUB member expanded beyond limit: {}", entry.name));
        }
        if entry.uncompressed_size != 0 && output.len() != entry.uncompressed_size {
            return Err(format!("EPUB member size mismatch: {}", entry.name));
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: String,
}

/// Parse one EPUB on a short-lived dedicated worker stack. The Reader keeps
/// its existing synchronous staged-loading contract, while archive parsing,
/// DEFLATE expansion and XHTML flattening no longer consume the firmware main
/// task's 16 KB stack budget.
pub fn open_epub_on_worker(path: impl AsRef<Path>) -> Result<EpubDocument, String> {
    let path = path.as_ref().to_path_buf();
    log::info!(
        "rustmix-wave=epub-parser-worker status=starting stack-bytes={}",
        EPUB_PARSER_WORKER_STACK_BYTES
    );
    let worker = std::thread::Builder::new()
        .name("epub-parser".into())
        .stack_size(EPUB_PARSER_WORKER_STACK_BYTES)
        .spawn(move || open_epub(path))
        .map_err(|error| {
            let message = format!("EPUB parser worker start failed: {error}");
            log::warn!("rustmix-wave=epub-parser-worker status=start-failed error={message}");
            message
        })?;
    let result = worker.join().map_err(|_| {
        let message = "EPUB parser worker panicked".to_string();
        log::warn!("rustmix-wave=epub-parser-worker status=panicked");
        message
    })?;
    match &result {
        Ok(document) => log::info!(
            "rustmix-wave=epub-parser-worker status=completed spine-items={} toc-entries={} text-bytes={}",
            document.spine_count,
            document.toc.len(),
            document.text_size_bytes()
        ),
        Err(error) => log::warn!("rustmix-wave=epub-parser-worker status=failed error={error}"),
    }
    result
}

/// Read only the OPF title on a lightweight bounded worker stack. Library scans
/// remain safe on the firmware main task and fall back to the FAT filename when
/// metadata cannot be read.
pub fn read_epub_title_on_worker(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref().to_path_buf();
    let worker = std::thread::Builder::new()
        .name("epub-title".into())
        .stack_size(EPUB_TITLE_WORKER_STACK_BYTES)
        .spawn(move || read_epub_title(path))
        .map_err(|error| format!("EPUB title worker start failed: {error}"))?;
    worker
        .join()
        .map_err(|_| "EPUB title worker panicked".to_string())?
}

/// Read one OPF metadata title without flattening the spine.
#[inline(never)]
pub fn read_epub_title(path: impl AsRef<Path>) -> Result<String, String> {
    let archive = ZipArchive::open(path)?;
    let (_, package, _) = epub_package(&archive)?;
    Ok(package_title(&package))
}

fn epub_package(archive: &ZipArchive) -> Result<(String, String, String), String> {
    let container = utf8_member(archive, "META-INF/container.xml")?;
    let rootfile = first_open_tag(&container, "rootfile")
        .and_then(|tag| attribute(tag, "full-path"))
        .ok_or_else(|| "EPUB container rootfile missing".to_string())?;
    let package_path = normalize_archive_path("", &rootfile);
    let package = utf8_member(archive, &package_path)?;
    let package_dir = archive_parent(&package_path);
    Ok((package_path, package, package_dir))
}

fn package_title(package: &str) -> String {
    first_element_text(package, "title")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Untitled EPUB".into())
}

/// Open one EPUB archive and index its chapters and navigation. Each chapter
/// is flattened once to measure it; none of the text is kept.
#[inline(never)]
pub fn open_epub(path: impl AsRef<Path>) -> Result<EpubDocument, String> {
    let path = path.as_ref();
    let archive = ZipArchive::open(path)?;
    let (_, package, package_dir) = epub_package(&archive)?;
    let title = package_title(&package);
    let language = first_element_text(&package, "language")
        .map(|tag| tag.trim().to_string())
        .unwrap_or_default();

    let manifest = parse_manifest(&package)?;
    let spine_ids = parse_spine_ids(&package)?;
    if spine_ids.is_empty() {
        return Err("EPUB spine is empty".into());
    }
    let links = parse_navigation_links(&archive, &package, &package_dir, &manifest)?;
    let wanted = wanted_fragments(&links);

    let mut pacer = Pacer::start();
    let mut text_size = 0_u64;
    let mut targets = BTreeMap::new();
    let mut chapters = Vec::new();
    for (spine_index, idref) in spine_ids.iter().enumerate() {
        let item = manifest
            .get(idref)
            .ok_or_else(|| format!("EPUB spine item missing from manifest: {idref}"))?;
        let member = normalize_archive_path(&package_dir, &item.href);
        let xhtml = utf8_member(&archive, &member)?;
        let (text, anchors) = flatten_xhtml(&xhtml, wanted.get(&member));
        pacer.pace();
        if text.is_empty() {
            continue;
        }
        if text_size > 0 {
            text_size += 2;
        }
        let offset = text_size;
        targets.insert(member.clone(), (spine_index, offset));
        for (id, local) in anchors {
            let at = offset + local as u64;
            targets.insert(format!("{member}#{id}"), (spine_index, at));
        }
        text_size += text.len() as u64;
        if text_size > EPUB_TEXT_LIMIT {
            return Err("EPUB text exceeds the size limit".into());
        }
        chapters.push(EpubChapter {
            number: chapters.len() + 1,
            label: fallback_chapter_label(&xhtml, spine_index),
            text_offset: offset,
            text_end_offset: text_size,
            spine_index,
            member,
        });
    }
    if chapters.is_empty() {
        return Err("EPUB spine did not contain readable text".into());
    }

    let mut toc = resolve_links(&links, &targets);
    if toc.is_empty() {
        toc = chapters
            .iter()
            .take(EPUB_TOC_LIMIT)
            .map(|chapter| EpubTocEntry {
                label: chapter.label.clone(),
                text_offset: chapter.text_offset,
                spine_index: chapter.spine_index,
            })
            .collect();
    }
    dedupe_toc(&mut toc);
    toc.truncate(EPUB_TOC_LIMIT);
    Ok(EpubDocument {
        path: path.to_string_lossy().into_owned(),
        title,
        language,
        text_size,
        toc,
        chapters,
        spine_count: spine_ids.len(),
    })
}

fn parse_manifest(package: &str) -> Result<BTreeMap<String, ManifestItem>, String> {
    let mut manifest = BTreeMap::new();
    for tag in open_tags(package, "item")
        .into_iter()
        .take(EPUB_MANIFEST_LIMIT)
    {
        let Some(id) = attribute(tag, "id") else {
            continue;
        };
        let Some(href) = attribute(tag, "href") else {
            continue;
        };
        let media_type = attribute(tag, "media-type").unwrap_or_default();
        let properties = attribute(tag, "properties").unwrap_or_default();
        manifest.insert(
            id.clone(),
            ManifestItem {
                id,
                href,
                media_type,
                properties,
            },
        );
    }
    if manifest.is_empty() {
        return Err("EPUB manifest is empty".into());
    }
    Ok(manifest)
}

fn parse_spine_ids(package: &str) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for tag in open_tags(package, "itemref")
        .into_iter()
        .take(EPUB_SPINE_LIMIT)
    {
        if let Some(idref) = attribute(tag, "idref") {
            ids.push(idref);
        }
    }
    Ok(ids)
}

fn parse_navigation_links(
    archive: &ZipArchive,
    package: &str,
    package_dir: &str,
    manifest: &BTreeMap<String, ManifestItem>,
) -> Result<Vec<NavigationLink>, String> {
    if let Some(nav) = manifest.values().find(|item| {
        item.properties
            .split_whitespace()
            .any(|value| value == "nav")
    }) {
        let member = normalize_archive_path(package_dir, &nav.href);
        let nav_text = utf8_member(archive, &member)?;
        let base = archive_parent(&member);
        let links = nav_links(toc_nav(&nav_text), &base);
        if !links.is_empty() {
            return Ok(links);
        }
    }

    let spine_toc = first_open_tag(package, "spine").and_then(|tag| attribute(tag, "toc"));
    let ncx = spine_toc
        .as_ref()
        .and_then(|id| manifest.get(id))
        .or_else(|| {
            manifest
                .values()
                .find(|item| item.media_type == "application/x-dtbncx+xml")
        });
    if let Some(ncx) = ncx {
        let member = normalize_archive_path(package_dir, &ncx.href);
        let ncx_text = utf8_member(archive, &member)?;
        let base = archive_parent(&member);
        return Ok(ncx_links(&ncx_text, &base));
    }
    Ok(Vec::new())
}

/// One TOC link before its target is located in the flattened text.
#[derive(Clone, Debug, Eq, PartialEq)]
struct NavigationLink {
    label: String,
    member: String,
    fragment: Option<String>,
}

fn navigation_link(base: &str, href: &str, label: &str) -> NavigationLink {
    let fragment = href
        .split_once('#')
        .map(|(_, fragment)| percent_decode(fragment))
        .filter(|fragment| !fragment.is_empty());
    NavigationLink {
        label: label.to_string(),
        member: normalize_archive_path(base, href),
        fragment,
    }
}

/// The `epub:type="toc"` nav of an EPUB3 navigation document, so page lists
/// and landmarks stay out of the TOC.
fn toc_nav(html: &str) -> &str {
    let mut cursor = 0;
    while let Some(start_rel) = html[cursor..].find("<nav") {
        let start = cursor + start_rel;
        let Some(end_rel) = html[start..].find('>') else {
            break;
        };
        let kinds = attribute(&html[start + 1..start + end_rel], "type").unwrap_or_default();
        if kinds.split_whitespace().any(|kind| kind == "toc") {
            let end = html[start..]
                .find("</nav>")
                .map_or(html.len(), |close| start + close);
            return &html[start..end];
        }
        cursor = start + end_rel + 1;
    }
    html
}

fn nav_links(html: &str, base: &str) -> Vec<NavigationLink> {
    let mut links = Vec::new();
    let mut cursor = 0;
    while links.len() < EPUB_TOC_LIMIT {
        let Some(start_rel) = html[cursor..].find("<a") else {
            break;
        };
        let start = cursor + start_rel;
        let Some(open_end_rel) = html[start..].find('>') else {
            break;
        };
        let open_end = start + open_end_rel;
        let tag = &html[start + 1..open_end];
        let Some(href) = attribute(tag, "href") else {
            cursor = open_end + 1;
            continue;
        };
        let Some(close_rel) = html[open_end + 1..].find("</a>") else {
            break;
        };
        let close = open_end + 1 + close_rel;
        let label = html_to_text(&html[open_end + 1..close]);
        links.push(navigation_link(base, &href, label.trim()));
        cursor = close + 4;
    }
    links
}

fn ncx_links(ncx: &str, base: &str) -> Vec<NavigationLink> {
    let mut links = Vec::new();
    let mut cursor = 0;
    while links.len() < EPUB_TOC_LIMIT {
        let Some(start_rel) = ncx[cursor..].find("<navPoint") else {
            break;
        };
        let start = cursor + start_rel;
        let body = start + "<navPoint".len();
        // A navPoint's own label and target come before any nested navPoint.
        let rest = &ncx[body..];
        let child = rest.find("<navPoint").unwrap_or(rest.len());
        let close = rest.find("</navPoint>").unwrap_or(rest.len());
        let block = &ncx[start..body + child.min(close)];
        let href = first_open_tag(block, "content").and_then(|tag| attribute(tag, "src"));
        let label = first_element_text(block, "text").unwrap_or_else(|| "Chapter".into());
        if let Some(href) = href {
            links.push(navigation_link(base, &href, label.trim()));
        }
        cursor = body;
    }
    links
}

/// Fragment ids each member must locate for the TOC.
fn wanted_fragments(links: &[NavigationLink]) -> BTreeMap<String, BTreeSet<String>> {
    let mut wanted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for link in links {
        if let Some(fragment) = &link.fragment {
            wanted
                .entry(link.member.clone())
                .or_default()
                .insert(fragment.clone());
        }
    }
    wanted
}

/// TOC entries for links whose member is a readable chapter. `targets` maps
/// `member` and `member#id` to a spine index and flattened text offset.
fn resolve_links(
    links: &[NavigationLink],
    targets: &BTreeMap<String, (usize, u64)>,
) -> Vec<EpubTocEntry> {
    let mut toc = Vec::new();
    for link in links {
        let anchored = link
            .fragment
            .as_ref()
            .and_then(|fragment| targets.get(&format!("{}#{fragment}", link.member)));
        let target = anchored.or_else(|| targets.get(&link.member));
        let Some(&(spine_index, text_offset)) = target else {
            continue;
        };
        let label = if link.label.is_empty() {
            format!("Chapter {}", spine_index + 1)
        } else {
            link.label.clone()
        };
        toc.push(EpubTocEntry {
            label,
            text_offset,
            spine_index,
        });
    }
    toc
}

fn dedupe_toc(toc: &mut Vec<EpubTocEntry>) {
    let mut unique = Vec::new();
    for entry in toc.drain(..) {
        if unique.iter().any(|existing: &EpubTocEntry| {
            existing.text_offset == entry.text_offset && existing.label == entry.label
        }) {
            continue;
        }
        unique.push(entry);
    }
    *toc = unique;
}

fn fallback_chapter_label(xhtml: &str, spine_index: usize) -> String {
    for tag in ["h1", "h2", "h3", "title"] {
        if let Some(label) = first_element_text(xhtml, tag).filter(|value| !value.is_empty()) {
            return label;
        }
    }
    format!("Chapter {}", spine_index + 1)
}

fn utf8_member(archive: &ZipArchive, name: &str) -> Result<String, String> {
    let bytes = archive.extract(name)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Flattened text of one archive member, exactly as `open_epub` measured it.
fn read_chapter_text(path: &Path, member: &str) -> Result<String, String> {
    let archive = ZipArchive::open(path)?;
    Ok(html_to_text(&utf8_member(&archive, member)?))
}

fn open_error(error: std::io::Error) -> String {
    format!("EPUB open failed: {error}")
}

fn read_error(error: std::io::Error) -> String {
    format!("EPUB read failed: {error}")
}

fn read_at(mut file: &File, offset: u64, len: usize) -> Result<Vec<u8>, String> {
    file.seek(SeekFrom::Start(offset)).map_err(read_error)?;
    let mut bytes = vec![0_u8; len];
    file.read_exact(&mut bytes).map_err(read_error)?;
    Ok(bytes)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let start = bytes.len().saturating_sub(65_557);
    (start..=bytes.len() - 22)
        .rev()
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(&[0x50_u8, 0x4B, 0x05, 0x06][..]))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "EPUB ZIP truncated u16".to_string())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "EPUB ZIP truncated u32".to_string())?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn archive_parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or("".into(), |(parent, _)| parent.into())
}

fn normalize_archive_path(base: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or("");
    let decoded = percent_decode(href);
    let raw = if decoded.starts_with('/') || base.is_empty() {
        decoded.trim_start_matches('/').to_string()
    } else {
        format!("{base}/{decoded}")
    };
    let mut parts = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    parts.join("/")
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                output.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn first_open_tag<'a>(xml: &'a str, local_name: &str) -> Option<&'a str> {
    open_tags(xml, local_name).into_iter().next()
}

fn open_tags<'a>(xml: &'a str, local_name: &str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let mut cursor = 0;
    while cursor < xml.len() {
        let Some(start_rel) = xml[cursor..].find('<') else {
            break;
        };
        let start = cursor + start_rel;
        let Some(end_rel) = xml[start + 1..].find('>') else {
            break;
        };
        let end = start + 1 + end_rel;
        let tag = &xml[start + 1..end];
        let trimmed = tag.trim_start();
        if !trimmed.starts_with('/') && !trimmed.starts_with('!') && !trimmed.starts_with('?') {
            let name = trimmed
                .split(|character: char| character.is_whitespace() || character == '/')
                .next()
                .unwrap_or("");
            if name.rsplit(':').next() == Some(local_name) {
                tags.push(tag);
            }
        }
        cursor = end + 1;
    }
    tags
}

fn attribute(tag: &str, key: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut cursor = 0;

    // Skip the opening element name before scanning attribute tokens. Without
    // this boundary, `<rootfile full-path='...'>` consumes `full-path` while
    // recovering from the non-attribute `rootfile` token.
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'/' {
        cursor += 1;
    }

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'/' {
            break;
        }

        let start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'/'
        {
            cursor += 1;
        }
        let name = &tag[start..cursor];
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'/'
            {
                cursor += 1;
            }
            continue;
        }

        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || (bytes[cursor] != b'\'' && bytes[cursor] != b'"') {
            while cursor < bytes.len()
                && !bytes[cursor].is_ascii_whitespace()
                && bytes[cursor] != b'/'
            {
                cursor += 1;
            }
            continue;
        }

        let quote = bytes[cursor];
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != quote {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }
        let value = &tag[value_start..cursor];
        cursor += 1;
        if name.rsplit(':').next() == Some(key) {
            return Some(decode_entities(value));
        }
    }
    None
}

fn first_element_text(xml: &str, local_name: &str) -> Option<String> {
    let mut cursor = 0;
    while cursor < xml.len() {
        let Some(start_rel) = xml[cursor..].find('<') else {
            return None;
        };
        let start = cursor + start_rel;
        let Some(end_rel) = xml[start + 1..].find('>') else {
            return None;
        };
        let end = start + 1 + end_rel;
        let tag = &xml[start + 1..end];
        let trimmed = tag.trim_start();
        let name = trimmed
            .split(|character: char| character.is_whitespace() || character == '/')
            .next()
            .unwrap_or("");
        if !trimmed.starts_with('/') && name.rsplit(':').next() == Some(local_name) {
            let close = format!("</{name}>");
            if let Some(close_rel) = xml[end + 1..].find(&close) {
                return Some(html_to_text(&xml[end + 1..end + 1 + close_rel]));
            }
        }
        cursor = end + 1;
    }
    None
}

/// Convert XHTML into bounded, paragraph-aware reflowable UTF-8 text.
pub fn html_to_text(html: &str) -> String {
    flatten_xhtml(html, None).0
}

/// `html_to_text`, also returning where each wanted element `id` starts in
/// the text.
fn flatten_xhtml(html: &str, wanted: Option<&BTreeSet<String>>) -> (String, Vec<(String, usize)>) {
    let mut output = String::new();
    let mut anchors = Vec::new();
    let mut cursor = 0;
    while cursor < html.len() {
        let rest = &html[cursor..];
        if rest.starts_with('<') {
            let Some(end_rel) = rest.find('>') else { break };
            let tag = rest[1..end_rel].trim();
            let closing = tag.starts_with('/');
            let name = tag
                .trim_start_matches('/')
                .split(|character: char| character.is_whitespace() || character == '/')
                .next()
                .unwrap_or("")
                .rsplit(':')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !closing && SKIPPED_ELEMENTS.contains(&name.as_str()) && !tag.ends_with('/') {
                if let Some(skip) = element_end(rest, &name) {
                    cursor += skip;
                    continue;
                }
            }
            if matches!(
                name.as_str(),
                "p" | "div"
                    | "section"
                    | "article"
                    | "blockquote"
                    | "li"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "br"
            ) {
                push_newline(&mut output);
                if closing || name == "br" {
                    push_newline(&mut output);
                }
            }
            if let Some(id) = wanted_id(tag, closing, wanted) {
                anchors.push((id, output.len()));
            }
            cursor += end_rel + 1;
            continue;
        }
        if rest.starts_with('&') {
            if let Some(end_rel) = rest.find(';').filter(|value| *value <= 12) {
                let decoded = decode_entity(&rest[1..end_rel]);
                for character in decoded.chars() {
                    push_text_character(&mut output, character);
                }
                cursor += end_rel + 1;
                continue;
            }
        }
        let character = rest.chars().next().unwrap_or(' ');
        push_text_character(&mut output, character);
        cursor += character.len_utf8();
    }
    let text = output.trim().to_string();
    for (_, offset) in &mut anchors {
        *offset = (*offset).min(text.len());
    }
    (text, anchors)
}

fn wanted_id(tag: &str, closing: bool, wanted: Option<&BTreeSet<String>>) -> Option<String> {
    let wanted = wanted.filter(|_| !closing)?;
    attribute(tag, "id").filter(|id| wanted.contains(id))
}

/// Bytes from `<name ...>` at the start of `html` through its closing tag.
fn element_end(html: &str, name: &str) -> Option<usize> {
    let closing = format!("</{name}");
    let bytes = html.as_bytes();
    let mut cursor = 0;
    while let Some(found) = find_ignoring_case(&bytes[cursor..], closing.as_bytes()) {
        let after = cursor + found + closing.len();
        if bytes
            .get(after)
            .is_some_and(|byte| *byte == b'>' || byte.is_ascii_whitespace())
        {
            return html[after..].find('>').map(|end| after + end + 1);
        }
        cursor = after;
    }
    None
}

fn find_ignoring_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

fn push_text_character(output: &mut String, character: char) {
    if character == '\r' {
        return;
    }
    if character == '\n' || character == '\t' || character.is_whitespace() {
        if !output.ends_with(' ') && !output.ends_with('\n') && !output.is_empty() {
            output.push(' ');
        }
    } else {
        output.push(character);
    }
}

fn push_newline(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
    if !output.ends_with("\n\n") && !output.is_empty() {
        output.push('\n');
    }
}

fn decode_entities(value: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < value.len() {
        let rest = &value[cursor..];
        if rest.starts_with('&') {
            if let Some(end_rel) = rest.find(';').filter(|value| *value <= 12) {
                output.push_str(&decode_entity(&rest[1..end_rel]));
                cursor += end_rel + 1;
                continue;
            }
        }
        let character = rest.chars().next().unwrap_or(' ');
        output.push(character);
        cursor += character.len_utf8();
    }
    output
}

fn decode_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        "nbsp" => " ".into(),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| "?".into(), |character| character.to_string())
        }
        value if value.starts_with('#') => value[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map_or_else(|| "?".into(), |character| character.to_string()),
        _ => "?".into(),
    }
}

#[cfg(test)]
fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend(value.to_le_bytes());
}

#[cfg(test)]
fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend(value.to_le_bytes());
}

/// Uncompressed ZIP archive holding `entries`, for tests.
#[cfg(test)]
pub(crate) fn stored_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut central = Vec::new();
    for (name, body) in entries {
        let offset = output.len() as u32;
        push_u32(&mut output, 0x0403_4B50);
        push_u16(&mut output, 20);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u32(&mut output, 0);
        push_u32(&mut output, body.len() as u32);
        push_u32(&mut output, body.len() as u32);
        push_u16(&mut output, name.len() as u16);
        push_u16(&mut output, 0);
        output.extend(name.as_bytes());
        output.extend(body.as_bytes());

        push_u32(&mut central, 0x0201_4B50);
        push_u16(&mut central, 20);
        push_u16(&mut central, 20);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, 0);
        push_u32(&mut central, body.len() as u32);
        push_u32(&mut central, body.len() as u32);
        push_u16(&mut central, name.len() as u16);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, 0);
        push_u32(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let central_offset = output.len() as u32;
    let central_size = central.len() as u32;
    output.extend(central);
    push_u32(&mut output, 0x0605_4B50);
    push_u16(&mut output, 0);
    push_u16(&mut output, 0);
    push_u16(&mut output, entries.len() as u16);
    push_u16(&mut output, entries.len() as u16);
    push_u32(&mut output, central_size);
    push_u32(&mut output, central_offset);
    push_u16(&mut output, 0);
    output
}

/// EPUB with one titled chapter per body, for tests.
#[cfg(test)]
pub(crate) fn sample_epub(bodies: &[String]) -> Vec<u8> {
    let mut manifest = String::new();
    let mut spine = String::new();
    let mut chapters = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        let number = index + 1;
        manifest.push_str(&format!("<item id='c{number}' href='c{number}.xhtml'/>"));
        spine.push_str(&format!("<itemref idref='c{number}'/>"));
        let content = format!("<h1>Capítulo {number}</h1><p>{body}</p>");
        let head = "<html><head><title>Libro</title></head>";
        let xhtml = format!("{head}<body>{content}</body></html>");
        chapters.push((format!("OEBPS/c{number}.xhtml"), xhtml));
    }
    let manifest = format!("<manifest>{manifest}</manifest>");
    let spine = format!("<spine>{spine}</spine>");
    let metadata = "<metadata><dc:language>es</dc:language></metadata>";
    let package = format!("<package>{metadata}{manifest}{spine}</package>");
    let container = "<container><rootfile full-path='OEBPS/book.opf'/></container>";
    let mut entries = vec![
        ("META-INF/container.xml", container),
        ("OEBPS/book.opf", package.as_str()),
    ];
    for (name, xhtml) in &chapters {
        entries.push((name.as_str(), xhtml.as_str()));
    }
    stored_zip(&entries)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        attribute, first_open_tag, html_to_text, ncx_links, open_epub, open_epub_on_worker,
        read_epub_title_on_worker, sample_epub, stored_zip, EPUB_PARSER_WORKER_STACK_BYTES,
        EPUB_TITLE_WORKER_STACK_BYTES,
    };

    fn temp_epub(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-{name}-{nonce}.epu"))
    }

    #[test]
    fn xml_attribute_tokenizer_reads_attributes_after_element_name() {
        let tag =
            "rootfile full-path='OEBPS/book.opf' media-type=\"application/oebps-package+xml\"/";
        assert_eq!(
            attribute(tag, "full-path").as_deref(),
            Some("OEBPS/book.opf")
        );
        assert_eq!(
            attribute(tag, "media-type").as_deref(),
            Some("application/oebps-package+xml")
        );
    }

    #[test]
    fn container_rootfile_lookup_ignores_plural_wrapper() {
        let container =
            "<container><rootfiles><rootfile full-path='OEBPS/book.opf'/></rootfiles></container>";
        let rootfile =
            first_open_tag(container, "rootfile").and_then(|tag| attribute(tag, "full-path"));
        assert_eq!(rootfile.as_deref(), Some("OEBPS/book.opf"));
    }

    #[test]
    fn parser_worker_stack_budget_is_explicit() {
        assert_eq!(EPUB_PARSER_WORKER_STACK_BYTES, 64 * 1024);
    }

    #[test]
    fn title_worker_uses_a_smaller_bounded_stack() {
        assert_eq!(EPUB_TITLE_WORKER_STACK_BYTES, 32 * 1024);
        assert!(EPUB_TITLE_WORKER_STACK_BYTES < EPUB_PARSER_WORKER_STACK_BYTES);
    }

    #[test]
    fn flattens_xhtml_into_reflowable_paragraphs() {
        assert_eq!(
            html_to_text("<h1>Title</h1><p>Hello &amp; goodbye.</p>"),
            "Title\n\nHello & goodbye."
        );
    }

    #[test]
    fn opens_stored_epub_manifest_spine_and_nav_toc() {
        let path = temp_epub("stored");
        let bytes = stored_zip(&[
            ("META-INF/container.xml", "<container><rootfiles><rootfile full-path='OEBPS/book.opf'/></rootfiles></container>"),
            ("OEBPS/book.opf", "<package><metadata><dc:title>Sample EPUB</dc:title></metadata><manifest><item id='nav' href='nav.xhtml' media-type='application/xhtml+xml' properties='nav'/><item id='c1' href='c1.xhtml' media-type='application/xhtml+xml'/><item id='c2' href='c2.xhtml' media-type='application/xhtml+xml'/></manifest><spine><itemref idref='c1'/><itemref idref='c2'/></spine></package>"),
            ("OEBPS/nav.xhtml", "<nav><ol><li><a href='c1.xhtml'>Start</a></li><li><a href='c2.xhtml'>Second</a></li></ol></nav>"),
            ("OEBPS/c1.xhtml", "<html><body><h1>Start</h1><p>First chapter.</p></body></html>"),
            ("OEBPS/c2.xhtml", "<html><body><h1>Second</h1><p>Second chapter.</p></body></html>"),
        ]);
        fs::write(&path, bytes).unwrap();
        let epub = open_epub(&path).unwrap();
        let worker_epub = open_epub_on_worker(&path).unwrap();
        assert_eq!(worker_epub, epub);
        assert_eq!(epub.title, "Sample EPUB");
        assert_eq!(epub.language, "");
        assert_eq!(epub.spine_count, 2);
        assert_eq!(epub.chapters.len(), 2);
        assert_eq!(epub.chapters[0].number, 1);
        assert_eq!(epub.chapters[1].number, 2);
        assert_eq!(
            epub.chapter_for_offset(epub.chapters[1].text_offset)
                .unwrap()
                .number,
            2
        );
        assert_eq!(read_epub_title_on_worker(&path).unwrap(), "Sample EPUB");
        assert!(epub.chapter_text(0).unwrap().contains("First chapter."));
        assert!(epub.chapter_text(1).unwrap().contains("Second chapter."));
        assert_eq!(epub.toc.len(), 2);
        assert_eq!(epub.toc[0].label, "Start");
        assert!(epub.toc[1].text_offset > epub.toc[0].text_offset);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn chapter_offsets_match_text_extracted_on_demand() {
        let path = temp_epub("chapters");
        let bodies = ["Uno dos.", "Tres.", "Cuatro."].map(String::from);
        fs::write(&path, sample_epub(&bodies)).unwrap();
        let epub = open_epub(&path).unwrap();
        assert_eq!(epub.chapters.len(), 3);
        assert_eq!(epub.language, "es");
        let mut expected_offset = 0;
        for (index, chapter) in epub.chapters.iter().enumerate() {
            let text = epub.chapter_text(index).unwrap();
            assert!(text.starts_with(&format!("Capítulo {}", index + 1)));
            assert_eq!(chapter.text_offset, expected_offset);
            let end = chapter.text_offset + text.len() as u64;
            assert_eq!(end, chapter.text_end_offset);
            expected_offset = end + 2;
        }
        assert_eq!(epub.text_size_bytes(), epub.chapters[2].text_end_offset);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn skips_head_style_and_script_content() {
        let html = "<html><head><title>Libro</title><style>p{color:red}</style></head>\
                    <body><script>var x = 1;</script><p>Hola</p></body></html>";
        assert_eq!(html_to_text(html), "Hola");
    }

    #[test]
    fn nested_ncx_points_keep_every_child() {
        let ncx = "<navMap><navPoint><navLabel><text>Génesis</text></navLabel>\
                   <content src='gen.xhtml'/>\
                   <navPoint><navLabel><text>1</text></navLabel><content src='gen.xhtml#c1'/>\
                   </navPoint><navPoint><navLabel><text>2</text></navLabel>\
                   <content src='gen.xhtml#c2'/></navPoint></navPoint></navMap>";
        let labels: Vec<String> = ncx_links(ncx, "OEBPS")
            .into_iter()
            .map(|link| link.label)
            .collect();
        assert_eq!(labels, ["Génesis", "1", "2"]);
    }

    #[test]
    fn toc_fragments_point_inside_their_chapter() {
        let path = temp_epub("fragments");
        let nav = "<nav epub:type='toc'><a href='gen.xhtml#c1'>1</a><a href='gen.xhtml#c2'>2</a>\
                   </nav><nav epub:type='page-list'><a href='gen.xhtml'>p1</a></nav>";
        let package = "<package><manifest><item id='nav' href='nav.xhtml' properties='nav'/>\
                       <item id='gen' href='gen.xhtml'/></manifest>\
                       <spine><itemref idref='gen'/></spine></package>";
        let chapter = "<html><body><h1 id='c1'>Uno</h1><p>Texto uno.</p>\
                       <h1 id='c2'>Dos</h1><p>Texto dos.</p></body></html>";
        let bytes = stored_zip(&[
            (
                "META-INF/container.xml",
                "<container><rootfile full-path='book.opf'/></container>",
            ),
            ("book.opf", package),
            ("nav.xhtml", nav),
            ("gen.xhtml", chapter),
        ]);
        fs::write(&path, bytes).unwrap();
        let epub = open_epub(&path).unwrap();
        let labels: Vec<&str> = epub.toc.iter().map(|entry| entry.label.as_str()).collect();
        assert_eq!(labels, ["1", "2"]);
        let text = epub.chapter_text(0).unwrap();
        let second = epub.toc[1].text_offset as usize;
        assert!(text[second..].starts_with("Dos"));
        let _ = fs::remove_file(path);
    }
}

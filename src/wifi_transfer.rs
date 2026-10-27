//! Explicitly activated Wi-Fi SD-card transfer web portal.
//!
//! The portal is intentionally dormant after boot.  The user must enter
//! Settings > Network and select Start Wi-Fi Transfer.  The target-specific
//! HTTP server owns its own ESP-IDF task while the main loop retains display,
//! routing and sleep ownership.

use std::path::{Component, Path, PathBuf};

use crate::json_lite;
use crate::wifi_networks::{NetworkBook, ScanEntry};

/// Portal root.  Browser requests may never escape this subtree.
pub const WIFI_TRANSFER_ROOT: &str = "/sdcard/RUSTMIX";
/// The ESP-IDF HTTP server task exists only while transfer mode is enabled.
pub const WIFI_TRANSFER_SERVER_STACK_BYTES: usize = 24 * 1024;
/// Upload and download bodies stream through one bounded scratch buffer.
pub const WIFI_TRANSFER_STREAM_CHUNK_BYTES: usize = 4 * 1024;
/// Keep accidental huge uploads bounded for the first portal slice.
pub const WIFI_TRANSFER_MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;
/// Bound decoded browser paths before touching removable storage.
pub const WIFI_TRANSFER_MAX_PATH_BYTES: usize = 255;
/// Stop a forgotten transfer portal after ten minutes without HTTP traffic.
pub const WIFI_TRANSFER_INACTIVITY_SECONDS: u64 = 10 * 60;
/// The first slice uses the conventional LAN HTTP port.
pub const WIFI_TRANSFER_HTTP_PORT: u16 = 80;
/// One authenticated browser session code is shown on the e-paper screen.
pub const WIFI_TRANSFER_CODE_DIGITS: usize = 6;
/// Bound one directory listing to protect the HTTP task heap.
pub const WIFI_TRANSFER_MAX_DIRECTORY_ROWS: usize = 256;

/// User request handed from the hardware-independent UI into `main.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiTransferUiRequest {
    Start,
    /// The device's own Wi-Fi, to add a network from a phone.
    StartSetup,
    Stop,
}

/// Compact UI-facing lifecycle state.  This snapshot never contains a Wi-Fi
/// password and remains safe to render or log.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WifiTransferState {
    #[default]
    Off,
    Starting,
    Ready,
    Failed,
}

impl WifiTransferState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Starting => "STARTING",
            Self::Ready => "READY",
            Self::Failed => "FAILED",
        }
    }
}

/// Small main-loop snapshot updated when the server starts, stops or handles a
/// completed filesystem request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WifiTransferSnapshot {
    pub state: WifiTransferState,
    pub url: Option<String>,
    pub code: Option<String>,
    pub last_action: String,
    pub last_bytes: usize,
    pub error: Option<String>,
    /// Name of the device's own Wi-Fi while it waits for a phone to add a
    /// network; `url` is then the setup page and `code` the Wi-Fi password.
    pub access_point: Option<String>,
}

impl Default for WifiTransferSnapshot {
    fn default() -> Self {
        Self {
            state: WifiTransferState::Off,
            url: None,
            code: None,
            last_action: "Portal is off".into(),
            last_bytes: 0,
            error: None,
            access_point: None,
        }
    }
}

impl WifiTransferSnapshot {
    #[must_use]
    pub fn starting() -> Self {
        Self {
            state: WifiTransferState::Starting,
            last_action: "Starting LAN portal".into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn starting_setup() -> Self {
        Self {
            state: WifiTransferState::Starting,
            last_action: "Starting Wi-Fi setup".into(),
            access_point: Some(String::new()),
            ..Self::default()
        }
    }

    /// True from asking for the setup Wi-Fi until it stops.
    #[must_use]
    pub const fn is_setup(&self) -> bool {
        self.access_point.is_some() && self.is_active()
    }

    #[must_use]
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            state: WifiTransferState::Failed,
            last_action: "Portal start failed".into(),
            error: Some(error.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(
            self.state,
            WifiTransferState::Starting | WifiTransferState::Ready
        )
    }

    #[must_use]
    pub fn url_label(&self) -> &str {
        self.url.as_deref().unwrap_or("--")
    }

    #[must_use]
    pub fn code_label(&self) -> &str {
        self.code.as_deref().unwrap_or("------")
    }
}

/// Resolve an already percent-decoded portal path. Rejects traversal, names
/// FAT cannot store and protected configuration files. Returned paths always
/// stay beneath `/sdcard/RUSTMIX`.
pub fn resolve_portal_path(relative: &str) -> Result<PathBuf, &'static str> {
    if relative.len() > WIFI_TRANSFER_MAX_PATH_BYTES {
        return Err("path exceeds portal limit");
    }
    let trimmed = relative.trim_start_matches('/');
    let mut safe = PathBuf::from(WIFI_TRANSFER_ROOT);
    let mut normalized = Vec::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_str().ok_or("path is not UTF-8")?;
                if !is_portal_safe_name(name) {
                    return Err("names cannot use \\ / : * ? \" < > | or end in a dot or space");
                }
                normalized.push(name);
                safe.push(name);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("path traversal is blocked")
            }
        }
    }
    if is_protected_portal_path(&normalized.join("/")) {
        return Err("protected configuration file");
    }
    Ok(safe)
}

/// Configuration files the portal never lists, reads or replaces, so
/// credentials are not disclosed and live config is not swapped underneath
/// running services.
const PROTECTED_PORTAL_PATHS: [&str; 8] = [
    "WIFI.TXT",
    "ALARMS.TXT",
    "DISPLAY.TXT",
    "WEATHER.TXT",
    "VOICE/META.TXT",
    "VOICE/SETTINGS.TXT",
    "APPS/CALENDAR/EVENTS.TMP",
    "APPS/CALENDAR/EVENTS.BAK",
];

#[must_use]
pub fn is_protected_portal_path(relative: &str) -> bool {
    let parts: Vec<&str> = relative
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    let normalized = parts.join("/");
    PROTECTED_PORTAL_PATHS
        .iter()
        .any(|protected| may_name(&normalized, protected))
}

/// FatFs matches names without case. Compare ASCII the same way and let any
/// non-ASCII character match, since it might fold to the protected letter.
fn may_name(candidate: &str, protected: &str) -> bool {
    candidate.chars().count() == protected.len()
        && candidate
            .chars()
            .zip(protected.chars())
            .all(|(left, right)| !left.is_ascii() || left.eq_ignore_ascii_case(&right))
}

/// Characters FAT long file names cannot contain.
const FAT_RESERVED: &str = "\"*/:<>?\\|";

/// FatFs also drops trailing dots and spaces, which would let one name alias
/// another, so those are rejected too.
#[must_use]
pub fn is_portal_safe_name(name: &str) -> bool {
    let reserved = |character: char| character.is_control() || FAT_RESERVED.contains(character);
    !name.is_empty()
        && name.chars().count() <= 255
        && !name.starts_with(' ')
        && !name.ends_with(|character| character == '.' || character == ' ')
        && !name.chars().any(reserved)
}

/// Tiny query parser used by the portal API.  The firmware intentionally avoids
/// allocating a generic web framework.
#[must_use]
pub fn query_value(uri: &str, key: &str) -> Option<String> {
    let query = uri.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (candidate, value) = part.split_once('=')?;
        (candidate == key)
            .then(|| percent_decode(value).ok())
            .flatten()
    })
}

fn percent_decode(value: &str) -> Result<String, &'static str> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex(bytes[index + 1]).ok_or("invalid percent escape")?;
                let low = hex(bytes[index + 2]).ok_or("invalid percent escape")?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| "path is not UTF-8")
}

const fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

/// The networks heard, as `/api/wifi/scan` sends them.
#[must_use]
pub fn scan_json(found: &[ScanEntry]) -> String {
    let rows: Vec<String> = found
        .iter()
        .map(|entry| {
            format!(
                "{{\"ssid\":\"{}\",\"rssi\":{},\"secured\":{}}}",
                json_lite::escape(&entry.ssid),
                entry.rssi_dbm,
                entry.secured
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// The saved networks, as `/api/wifi` sends them: names only, never passwords.
#[must_use]
pub fn saved_json(book: &NetworkBook) -> String {
    let names: Vec<String> = book
        .networks()
        .iter()
        .map(|network| format!("\"{}\"", json_lite::escape(&network.ssid)))
        .collect();
    format!("{{\"available\":true,\"saved\":[{}]}}", names.join(","))
}

/// The setup Wi-Fi's name (`Wave-` and four hex digits) and its eight-digit
/// password, from two random words.
#[must_use]
pub fn setup_credentials(random_a: u32, random_b: u32) -> (String, String) {
    (
        format!("Wave-{:04X}", random_a & 0xFFFF),
        format!("{:08}", random_b % 100_000_000),
    )
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use std::{
        fs::{self, File},
        io::{Read as StdRead, Write as StdWrite},
        path::Path,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use anyhow::{anyhow, bail, ensure, Context, Result};
    use embedded_svc::{http::Method, io::Write as _};
    use esp_idf_svc::http::server::{Configuration, EspHttpServer};
    use log::info;

    use super::{
        is_protected_portal_path, query_value, resolve_portal_path, saved_json, scan_json,
        WifiTransferSnapshot, WifiTransferState, WIFI_TRANSFER_HTTP_PORT,
        WIFI_TRANSFER_INACTIVITY_SECONDS, WIFI_TRANSFER_MAX_DIRECTORY_ROWS,
        WIFI_TRANSFER_MAX_UPLOAD_BYTES, WIFI_TRANSFER_ROOT, WIFI_TRANSFER_SERVER_STACK_BYTES,
        WIFI_TRANSFER_STREAM_CHUNK_BYTES,
    };
    use crate::ai_keys::{espidf::AiKeyStore, validate_key, AiKeySlot, AI_KEY_MAX_CHARS};
    use crate::wifi_networks::{self, espidf::WifiNetworkStore};

    const PORTAL_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Wave Transfer</title><style>
body{font-family:sans-serif;max-width:900px;margin:2rem auto;padding:0 1rem}button,input{padding:.65rem;margin:.2rem}pre{white-space:pre-wrap;background:#f4f4f4;padding:1rem}table{width:100%;border-collapse:collapse}td,th{border-bottom:1px solid #ddd;padding:.5rem;text-align:left}
</style></head><body><h1>Wave Wi-Fi Transfer</h1>
<p>LAN-only SD portal rooted at <code>/RUSTMIX</code>. Long file names work; avoid \ / : * ? " &lt; &gt; |.</p>
<label>Session code <input id="code" maxlength="6"><button onclick="saveCode()">Unlock</button></label>
<p><button onclick="loadList('/')">Home</button><button onclick="loadList(current)">Refresh</button></p>
<p>Path: <code id="path">/</code></p><div id="list"></div>
<h2>Photos</h2>
<p>Pick photos on this phone or computer, in any format the browser opens (iPhone photos included). The browser turns each one upright, shrinks it to at most 1600 pixels on its long side and sends it as a standard JPEG to <code>/PHOTOS</code>. The device prepares it for the screen the next time Photos opens.</p>
<input id="photos" type="file" accept="image/*" multiple><button onclick="sendPhotos()">Add photos</button>
<h2>Upload</h2><input id="file" type="file"><input id="name" placeholder="Optional new name"><button onclick="upload()">Upload</button>
<h2>Folder</h2><input id="folder" placeholder="Folder name"><button onclick="mkdir()">Create folder</button>
<h2>AI keys</h2>
<p>For Voice Notes. A key is stored encrypted on the device, never written to the SD card and never shown again.</p>
<p id="keys">Unlock to see which keys are set.</p>
<select id="slot"><option value="transcription">Transcription key</option><option value="summary">Summary key</option></select>
<input id="key" type="password" autocomplete="off" placeholder="Paste the API key"><button onclick="saveKey()">Save key</button><button onclick="clearKey()">Remove key</button>
<h2>Wi-Fi networks</h2>
<p>The device joins the strongest of these that it hears, after the one in <code>WIFI.TXT</code>. Passwords are stored encrypted on the device, never on the SD card and never shown again. Save your phone's hotspot here to use it away from home.</p>
<p id="nets">Unlock to see the saved networks.</p>
<p><button onclick="scanNets()">Scan nearby</button> <select id="found"></select></p>
<input id="ssid" placeholder="Network name"><input id="pass" type="password" autocomplete="off" placeholder="Password (empty if open)"><button onclick="saveNet()">Save network</button>
<pre id="status">Enter the six-digit code displayed on the device.</pre>
<script>
let current='/'; const status=t=>document.getElementById('status').textContent=t;
const code=()=>localStorage.waveCode||document.getElementById('code').value;
function saveCode(){localStorage.waveCode=document.getElementById('code').value;loadList(current);keys()}
const enc=s=>encodeURIComponent(s); const join=n=>(current==='/'?'/':current+'/')+n;
async function api(url,opt){let r=await fetch(url+(url.includes('?')?'&':'?')+'code='+enc(code()),opt);let t=await r.text();if(!r.ok)throw Error(t);return t}
// File names come from the SD card, so they are only ever set as text, never parsed as HTML.
function cell(row,text){let c=row.insertCell();c.textContent=text;return c}
function button(label,action){let b=document.createElement('button');b.textContent=label;b.onclick=action;return b}
async function loadList(path){try{current=path;document.getElementById('path').textContent=path;let rows=JSON.parse(await api('/api/list?path='+enc(path)));
let table=document.createElement('table');let head=table.createTHead().insertRow();for(let t of ['Name','Type','Size','Actions'])head.appendChild(document.createElement('th')).textContent=t;
let body=table.createTBody();if(path!=='/'){let r=body.insertRow();cell(r,'').appendChild(button('..',up));cell(r,'folder');cell(r,'');cell(r,'')}
for(let e of rows){let p=join(e.name);let r=body.insertRow();cell(r,e.name);cell(r,e.kind);cell(r,e.size);let a=cell(r,'');
if(e.kind==='folder'){a.appendChild(button('Open',()=>loadList(p)))}else{let l=document.createElement('a');l.href='/api/download?code='+enc(code())+'&path='+enc(p);l.textContent='Download';a.appendChild(l)}
a.appendChild(button('Rename',()=>renamePath(p)));a.appendChild(button('Delete',()=>del(p)))}
document.getElementById('list').replaceChildren(table);status('Ready')}catch(e){status(e.message)}}
function up(){let p=current.split('/').filter(Boolean);p.pop();loadList('/'+p.join('/'))}
async function upload(){try{let f=document.getElementById('file').files[0];let n=document.getElementById('name').value||f.name;await api('/api/upload?path='+enc(join(n)),{method:'POST',body:f});status('Upload complete');loadList(current)}catch(e){status(e.message)}}
const PHOTO_SIDE=1600;
function openImage(f){return new Promise((ok,no)=>{let u=URL.createObjectURL(f),i=new Image();i.onload=()=>{URL.revokeObjectURL(u);ok(i)};i.onerror=()=>{URL.revokeObjectURL(u);no(Error('this browser cannot open it'))};i.src=u})}
function draw(src,w,h){let c=document.createElement('canvas');c.width=w;c.height=h;let x=c.getContext('2d');x.fillStyle='#fff';x.fillRect(0,0,w,h);x.imageSmoothingEnabled=true;x.imageSmoothingQuality='high';x.drawImage(src,0,0,w,h);return c}
// Upright (the browser applies the EXIF turn), halved in steps so detail survives, then one last scale to the long side.
async function photoJpeg(f){let img;try{img=await createImageBitmap(f,{imageOrientation:'from-image'})}catch(e){img=await openImage(f)}
let src=img,w=img.width,h=img.height;while(Math.max(w,h)/2>=PHOTO_SIDE){w=Math.round(w/2);h=Math.round(h/2);src=draw(src,w,h)}
let s=Math.min(1,PHOTO_SIDE/Math.max(w,h));let c=draw(src,Math.max(1,Math.round(w*s)),Math.max(1,Math.round(h*s)));if(img.close)img.close();
return await new Promise((ok,no)=>c.toBlob(b=>b?ok(b):no(Error('could not be converted')),'image/jpeg',0.92))}
function two(n){return String(n).padStart(2,'0')}
function photoName(f,taken){let base=f.name.replace(/\.[^.]*$/,'').replace(/[\\/:*?"<>|\x00-\x1f]/g,'_').trim().slice(0,60);
if(!base||/^(image|photo|img)$/i.test(base)){let d=new Date(f.lastModified||Date.now());base='IMG_'+d.getFullYear()+two(d.getMonth()+1)+two(d.getDate())+'_'+two(d.getHours())+two(d.getMinutes())+two(d.getSeconds())}
let name=base+'.jpg',n=2;while(taken.has(name.toLowerCase())){name=base+'-'+n+'.jpg';n++}taken.add(name.toLowerCase());return name}
async function sendPhotos(){let files=[...document.getElementById('photos').files];if(!files.length){status('Pick one or more photos first.');return}
let taken=new Set();try{for(let e of JSON.parse(await api('/api/list?path='+enc('/PHOTOS'))))taken.add(e.name.toLowerCase())}catch(e){try{await api('/api/mkdir?path='+enc('/PHOTOS'),{method:'POST'})}catch(e2){}}
let done=0,failed=[];for(let [i,f] of files.entries()){try{status('Photo '+(i+1)+' of '+files.length+': preparing '+f.name+'...');let jpeg=await photoJpeg(f);let name=photoName(f,taken);
status('Photo '+(i+1)+' of '+files.length+': sending '+name+' ('+Math.round(jpeg.size/1024)+' KB)...');await api('/api/upload?path='+enc('/PHOTOS/'+name),{method:'POST',body:jpeg});done++}catch(e){failed.push(f.name+': '+e.message)}}
document.getElementById('photos').value='';status(done+' of '+files.length+' photos added to /PHOTOS. Open Photos on the device to see them.'+(failed.length?'\nNot added:\n'+failed.join('\n'):''));if(current==='/PHOTOS')loadList(current)}
async function mkdir(){try{await api('/api/mkdir?path='+enc(join(document.getElementById('folder').value)),{method:'POST'});status('Folder created');loadList(current)}catch(e){status(e.message)}}
async function renamePath(p){try{let n=prompt('New name');if(!n)return;let parent=p.substring(0,p.lastIndexOf('/'))||'/';let to=(parent==='/'?'/':parent+'/')+n;await api('/api/rename?from='+enc(p)+'&to='+enc(to),{method:'POST'});status('Renamed');loadList(current)}catch(e){status(e.message)}}
async function del(p){try{await api('/api/delete?path='+enc(p),{method:'POST'});status('Deleted');loadList(current)}catch(e){status(e.message)}}
// Keys only go to the device; it answers with which ones are set, never the keys.
const slot=()=>enc(document.getElementById('slot').value);
async function keys(){try{let k=JSON.parse(await api('/api/ai-keys'));document.getElementById('keys').textContent=k.available?'Transcription key: '+(k.transcription?'set':'not set')+' · Summary key: '+(k.summary?'set':'not set'):'Key storage is unavailable on this device.'}catch(e){}}
async function saveKey(){try{let i=document.getElementById('key');await api('/api/ai-keys?slot='+slot(),{method:'POST',body:i.value});i.value='';status('Key saved');keys()}catch(e){status(e.message)}}
async function clearKey(){try{await api('/api/ai-keys?slot='+slot()+'&clear=1',{method:'POST'});status('Key removed');keys()}catch(e){status(e.message)}}
// The device answers with names only, never passwords.
async function nets(){try{let n=JSON.parse(await api('/api/wifi'));let box=document.getElementById('nets');box.replaceChildren();if(!n.available){box.textContent='Network storage is unavailable on this device.';return}
if(!n.saved.length){box.textContent='No saved networks.';return}
for(let s of n.saved){let row=document.createElement('div');row.append(s+' ',button('Remove',()=>dropNet(s)));box.append(row)}}catch(e){}}
async function scanNets(){try{status('Scanning, a few seconds...');let f=JSON.parse(await api('/api/wifi/scan'));let sel=document.getElementById('found');sel.replaceChildren();
for(let n of f){let o=document.createElement('option');o.value=n.ssid;o.textContent=n.ssid+' ('+n.rssi+' dBm'+(n.secured?'':', open')+')';sel.append(o)}
sel.onchange=()=>document.getElementById('ssid').value=sel.value;if(f.length)document.getElementById('ssid').value=f[0].ssid;status(f.length+' networks found')}catch(e){status(e.message)}}
async function saveNet(){try{let p=document.getElementById('pass');await api('/api/wifi?ssid='+enc(document.getElementById('ssid').value),{method:'POST',body:p.value});p.value='';status('Network saved');nets()}catch(e){status(e.message)}}
async function dropNet(s){try{await api('/api/wifi?clear=1&ssid='+enc(s),{method:'POST'});status('Network removed');nets()}catch(e){status(e.message)}}
loadList('/');keys();nets();
</script></body></html>"#;

    /// The saved-network routes, shared by the LAN portal (behind its session
    /// code) and the setup page (behind the device's own Wi-Fi password, so
    /// `code` is `None`). `cache` holds the scan made when setup started: a
    /// fresh scan while the phone is joined can drop its connection for a
    /// moment, so the page only rescans when asked (`fresh=1`).
    fn add_network_routes(
        server: &mut EspHttpServer<'static>,
        code: Option<String>,
        networks: Option<WifiNetworkStore>,
        shared: &Arc<Mutex<SharedStatus>>,
        cache: Option<Arc<Mutex<Vec<wifi_networks::ScanEntry>>>>,
    ) -> Result<()> {
        type Gate = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;
        let gate: Gate = Arc::new(move |uri: &str| match &code {
            Some(expected) => authenticate(uri, expected),
            None => Ok(()),
        });

        // The saved Wi-Fi networks: names only on the way out.
        let list_networks = networks.clone();
        let list_gate = Arc::clone(&gate);
        server.fn_handler("/api/wifi", Method::Get, move |request| {
            list_gate(request.uri())?;
            let body = match &list_networks {
                Some(store) => saved_json(&store.load()),
                None => "{\"available\":false,\"saved\":[]}".into(),
            };
            request.into_ok_response()?.write_all(body.as_bytes())?;
            Ok::<(), anyhow::Error>(())
        })?;

        // Networks in range: a scan of a few seconds, or the one setup made.
        let scan_gate = Arc::clone(&gate);
        server.fn_handler("/api/wifi/scan", Method::Get, move |request| {
            scan_gate(request.uri())?;
            let fresh = query_value(request.uri(), "fresh").is_some();
            let found = match &cache {
                Some(cached) if !fresh => {
                    cached.lock().map(|list| list.clone()).unwrap_or_default()
                }
                _ => {
                    let found = wifi_networks::espidf::scan()?;
                    if let Some(cached) = &cache {
                        if let Ok(mut list) = cached.lock() {
                            list.clone_from(&found);
                        }
                    }
                    found
                }
            };
            request
                .into_ok_response()?
                .write_all(scan_json(&found).as_bytes())?;
            Ok::<(), anyhow::Error>(())
        })?;

        // Save a network (the password is the body) or remove one.
        let network_shared = Arc::clone(shared);
        server.fn_handler("/api/wifi", Method::Post, move |mut request| {
            gate(request.uri())?;
            let store = networks
                .as_ref()
                .ok_or_else(|| anyhow!("Network storage is unavailable on this device"))?;
            let ssid = required_query(request.uri(), "ssid")?;
            let mut book = store.load();
            if query_value(request.uri(), "clear").is_some() {
                ensure!(book.remove(&ssid), "That network is not saved");
                store.save(&book)?;
                lock(&network_shared).touch(format!("Removed {ssid}"), 0);
                info!("rustmix-wave=wifi-network status=removed ssid={ssid}");
                request.into_ok_response()?.write_all(b"removed")?;
                return Ok(());
            }
            let mut body = [0_u8; 96];
            let mut total = 0;
            loop {
                let read = request.read(&mut body[total..])?;
                if read == 0 {
                    break;
                }
                total += read;
                if total == body.len() {
                    bail!("The password is too long.");
                }
            }
            let saved = std::str::from_utf8(&body[..total])
                .map_err(|_| anyhow!("The password is not text."))
                .and_then(|password| book.add(&ssid, password).map_err(|error| anyhow!(error)));
            body.fill(0);
            let outcome = saved?;
            store.save(&book)?;
            lock(&network_shared).touch(format!("Saved {ssid}"), 0);
            info!("rustmix-wave=wifi-network status=saved ssid={ssid} outcome={outcome:?}");
            request.into_ok_response()?.write_all(b"saved")?;
            Ok::<(), anyhow::Error>(())
        })?;
        Ok(())
    }

    const SETUP_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Wave Wi-Fi setup</title><style>
body{font-family:sans-serif;max-width:600px;margin:1.5rem auto;padding:0 1rem}button,input,select{padding:.7rem;margin:.2rem;font-size:1rem;max-width:100%}pre{white-space:pre-wrap;background:#f4f4f4;padding:1rem}
</style></head><body><h1>Wave Wi-Fi setup</h1>
<p>Pick the network the device should join (your phone's hotspot, a friend's Wi-Fi) and type its password. It is stored encrypted on the device and never shown again. The device tries all saved networks and joins the strongest it hears.</p>
<p><b>Adding this phone's hotspot?</b> While the phone is joined to this setup Wi-Fi it stops sharing its hotspot, so the hotspot may be missing from the list: type its name. Turn on Personal Hotspot first; on iPhone 12 or later also turn on <i>Maximize Compatibility</i>, because the device only hears 2.4&nbsp;GHz. Type the name exactly as the phone shows it in Settings &rsaquo; General &rsaquo; About &rsaquo; Name; an iPhone uses a curly apostrophe (Miguel&rsquo;s iPhone, not Miguel's iPhone). Then press Done on the device and keep the Personal Hotspot screen open until the device joins.</p>
<p><button onclick="scanNets(true)">Scan again</button></p>
<p><select id="found" size="6" style="width:100%"></select></p>
<input id="ssid" placeholder="Network name"><input id="pass" type="password" autocomplete="off" placeholder="Password (empty if open)"><button onclick="saveNet()">Save network</button>
<p id="nets"></p>
<pre id="status">Scanning...</pre>
<script>
const status=t=>document.getElementById('status').textContent=t;const enc=encodeURIComponent;
async function api(url,opt){let r=await fetch(url,opt);let t=await r.text();if(!r.ok)throw Error(t);return t}
async function scanNets(fresh){try{status(fresh?'Scanning, the page may pause a few seconds...':'Loading...');let f=JSON.parse(await api('/api/wifi/scan'+(fresh?'?fresh=1':'')));let sel=document.getElementById('found');sel.replaceChildren();
for(let n of f){let o=document.createElement('option');o.value=n.ssid;o.textContent=n.ssid+' ('+n.rssi+' dBm'+(n.secured?'':', open')+')';sel.append(o)}
sel.onchange=()=>document.getElementById('ssid').value=sel.value;status(f.length+' networks found. Pick one.')}catch(e){status(e.message)}}
async function saveNet(){try{let p=document.getElementById('pass');await api('/api/wifi?ssid='+enc(document.getElementById('ssid').value),{method:'POST',body:p.value});p.value='';status('Saved. Add another, or press Done on the device.');nets()}catch(e){status(e.message)}}
async function nets(){try{let n=JSON.parse(await api('/api/wifi'));document.getElementById('nets').textContent=n.saved.length?'Saved: '+n.saved.join(', '):''}catch(e){}}
scanNets(false);nets();
</script></body></html>"#;

    #[derive(Debug)]
    struct SharedStatus {
        snapshot: WifiTransferSnapshot,
        last_activity: Instant,
    }

    impl SharedStatus {
        fn new(url: String, code: String) -> Self {
            Self {
                snapshot: WifiTransferSnapshot {
                    state: WifiTransferState::Ready,
                    url: Some(url),
                    code: Some(code),
                    last_action: "Portal ready".into(),
                    last_bytes: 0,
                    error: None,
                    access_point: None,
                },
                last_activity: Instant::now(),
            }
        }

        fn touch(&mut self, action: impl Into<String>, bytes: usize) {
            self.snapshot.last_action = action.into();
            self.snapshot.last_bytes = bytes;
            self.snapshot.error = None;
            self.last_activity = Instant::now();
        }
    }

    /// RAII wrapper.  Constructing starts ESP-IDF's dedicated HTTP task;
    /// dropping stops that task and frees its resources.
    pub struct WifiTransferServer {
        _server: EspHttpServer<'static>,
        shared: Arc<Mutex<SharedStatus>>,
    }

    impl WifiTransferServer {
        /// `keys` is the encrypted AI key store, when NVS opened.
        pub fn start(
            ipv4: &str,
            code: String,
            keys: Option<AiKeyStore>,
            networks: Option<WifiNetworkStore>,
        ) -> Result<Self> {
            let url = format!("http://{ipv4}/");
            let shared = Arc::new(Mutex::new(SharedStatus::new(url.clone(), code.clone())));
            let mut server = EspHttpServer::new(&Configuration {
                http_port: WIFI_TRANSFER_HTTP_PORT,
                stack_size: WIFI_TRANSFER_SERVER_STACK_BYTES,
                max_open_sockets: 2,
                max_sessions: 2,
                max_uri_handlers: 16,
                session_timeout: Duration::from_secs(60),
                ..Default::default()
            })?;

            server.fn_handler("/", Method::Get, move |request| {
                request
                    .into_ok_response()?
                    .write_all(PORTAL_HTML.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            let list_shared = Arc::clone(&shared);
            let list_code = code.clone();
            server.fn_handler("/api/list", Method::Get, move |request| {
                authenticate(request.uri(), &list_code)?;
                let relative = query_value(request.uri(), "path").unwrap_or_else(|| "/".into());
                let body = list_directory_json(&relative)?;
                lock(&list_shared).touch(format!("Listed {relative}"), body.len());
                request.into_ok_response()?.write_all(body.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            let download_shared = Arc::clone(&shared);
            let download_code = code.clone();
            server.fn_handler("/api/download", Method::Get, move |request| {
                authenticate(request.uri(), &download_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                let mut file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
                let mut response = request.into_ok_response()?;
                let mut buffer = [0_u8; WIFI_TRANSFER_STREAM_CHUNK_BYTES];
                let mut total = 0;
                loop {
                    let read = StdRead::read(&mut file, &mut buffer)?;
                    if read == 0 { break; }
                    response.write_all(&buffer[..read])?;
                    total += read;
                }
                lock(&download_shared).touch(format!("Downloaded {relative}"), total);
                info!("rustmix-wave=wifi-transfer-request method=GET route=download path={relative} bytes={total} status=completed");
                Ok::<(), anyhow::Error>(())
            })?;

            let upload_shared = Arc::clone(&shared);
            let upload_code = code.clone();
            server.fn_handler("/api/upload", Method::Post, move |mut request| {
                authenticate(request.uri(), &upload_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                let temporary = temporary_path(&path)?;
                if temporary.exists() {
                    fs::remove_file(&temporary)?;
                }
                let transfer_result = (|| -> Result<usize> {
                    let mut file = File::create(&temporary)
                        .with_context(|| format!("create {}", temporary.display()))?;
                    let mut buffer = [0_u8; WIFI_TRANSFER_STREAM_CHUNK_BYTES];
                    let mut total = 0;
                    loop {
                        let read = request.read(&mut buffer)?;
                        if read == 0 { break; }
                        total += read;
                        if total > WIFI_TRANSFER_MAX_UPLOAD_BYTES {
                            bail!("upload exceeds 64 MiB limit");
                        }
                        StdWrite::write_all(&mut file, &buffer[..read])?;
                    }
                    StdWrite::flush(&mut file)?;
                    Ok(total)
                })();
                let total = match transfer_result {
                    Ok(total) => total,
                    Err(error) => {
                        let _ = fs::remove_file(&temporary);
                        return Err(error);
                    }
                };
                commit_atomic_upload(&temporary, &path)?;
                lock(&upload_shared).touch(format!("Uploaded {relative}"), total);
                info!("rustmix-wave=wifi-transfer-request method=POST route=upload path={relative} bytes={total} status=completed");
                request.into_ok_response()?.write_all(b"uploaded")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let delete_shared = Arc::clone(&shared);
            let delete_code = code.clone();
            server.fn_handler("/api/delete", Method::Post, move |request| {
                authenticate(request.uri(), &delete_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                if path.is_dir() { fs::remove_dir(&path)?; } else { fs::remove_file(&path)?; }
                lock(&delete_shared).touch(format!("Deleted {relative}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=delete path={relative} status=completed");
                request.into_ok_response()?.write_all(b"deleted")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let mkdir_shared = Arc::clone(&shared);
            let mkdir_code = code.clone();
            server.fn_handler("/api/mkdir", Method::Post, move |request| {
                authenticate(request.uri(), &mkdir_code)?;
                let relative = required_query(request.uri(), "path")?;
                reject_protected(&relative)?;
                let path = resolve_portal_path(&relative).map_err(|error| anyhow!(error))?;
                fs::create_dir(&path)?;
                lock(&mkdir_shared).touch(format!("Created {relative}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=mkdir path={relative} status=completed");
                request.into_ok_response()?.write_all(b"created")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let rename_shared = Arc::clone(&shared);
            let rename_code = code.clone();
            server.fn_handler("/api/rename", Method::Post, move |request| {
                authenticate(request.uri(), &rename_code)?;
                let from = required_query(request.uri(), "from")?;
                let to = required_query(request.uri(), "to")?;
                reject_protected(&from)?;
                reject_protected(&to)?;
                let source = resolve_portal_path(&from).map_err(|error| anyhow!(error))?;
                let destination = resolve_portal_path(&to).map_err(|error| anyhow!(error))?;
                fs::rename(&source, &destination)?;
                lock(&rename_shared).touch(format!("Renamed {from}"), 0);
                info!("rustmix-wave=wifi-transfer-request method=POST route=rename from={from} to={to} status=completed");
                request.into_ok_response()?.write_all(b"renamed")?;
                Ok::<(), anyhow::Error>(())
            })?;

            let status_shared = Arc::clone(&shared);
            let status_code = code.clone();
            server.fn_handler("/api/status", Method::Get, move |request| {
                authenticate(request.uri(), &status_code)?;
                let snapshot = lock(&status_shared).snapshot.clone();
                let body = format!(
                    "{{\"state\":\"{}\",\"last_action\":\"{}\",\"last_bytes\":{}}}",
                    snapshot.state.label(),
                    json_escape(&snapshot.last_action),
                    snapshot.last_bytes
                );
                request.into_ok_response()?.write_all(body.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            // Which AI keys are set; never the keys themselves.
            let presence_code = code.clone();
            let presence_keys = keys.clone();
            server.fn_handler("/api/ai-keys", Method::Get, move |request| {
                authenticate(request.uri(), &presence_code)?;
                let presence = presence_keys
                    .as_ref()
                    .map(AiKeyStore::presence)
                    .unwrap_or_default();
                let body = format!(
                    "{{\"available\":{},\"transcription\":{},\"summary\":{}}}",
                    presence.available, presence.transcription, presence.summary
                );
                request.into_ok_response()?.write_all(body.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;

            // Save a key from the request body, or remove one with `clear`.
            let key_shared = Arc::clone(&shared);
            let key_code = code.clone();
            server.fn_handler("/api/ai-keys", Method::Post, move |mut request| {
                authenticate(request.uri(), &key_code)?;
                let store = keys
                    .as_ref()
                    .ok_or_else(|| anyhow!("Key storage is unavailable on this device"))?;
                let slot = required_query(request.uri(), "slot")?;
                let slot = AiKeySlot::from_marker(&slot).ok_or_else(|| anyhow!("unknown key"))?;
                if query_value(request.uri(), "clear").is_some() {
                    store.clear(slot)?;
                    lock(&key_shared).touch(format!("Removed the {} key", slot.marker()), 0);
                    info!("rustmix-wave=ai-key status=removed slot={}", slot.marker());
                    request.into_ok_response()?.write_all(b"removed")?;
                    return Ok(());
                }
                let mut body = [0_u8; AI_KEY_MAX_CHARS + 64];
                let mut total = 0;
                loop {
                    let read = request.read(&mut body[total..])?;
                    if read == 0 {
                        break;
                    }
                    total += read;
                    if total == body.len() {
                        bail!("The key is too long.");
                    }
                }
                let saved = std::str::from_utf8(&body[..total])
                    .map_err(|_| anyhow!("The key is not text."))
                    .and_then(|text| validate_key(text).map_err(|error| anyhow!(error)))
                    .and_then(|key| store.set(slot, key));
                body.fill(0);
                saved?;
                lock(&key_shared).touch(format!("Saved the {} key", slot.marker()), 0);
                info!("rustmix-wave=ai-key status=saved slot={}", slot.marker());
                request.into_ok_response()?.write_all(b"saved")?;
                Ok::<(), anyhow::Error>(())
            })?;

            add_network_routes(&mut server, Some(code.clone()), networks, &shared, None)?;

            info!("rustmix-wave=wifi-transfer-server status=ready url={url} root={WIFI_TRANSFER_ROOT} stack-bytes={WIFI_TRANSFER_SERVER_STACK_BYTES}");
            Ok(Self {
                _server: server,
                shared,
            })
        }

        /// The setup page on the device's own Wi-Fi (`ipv4` is its address).
        /// `scan` is what the radio heard before the access point went up.
        pub fn start_setup(
            ipv4: &str,
            ssid: &str,
            password: &str,
            networks: Option<WifiNetworkStore>,
            scan: Vec<wifi_networks::ScanEntry>,
        ) -> Result<Self> {
            let url = format!("http://{ipv4}/");
            let shared = Arc::new(Mutex::new(SharedStatus::new(url.clone(), password.into())));
            {
                let mut status = lock(&shared);
                status.snapshot.access_point = Some(ssid.into());
                status.snapshot.last_action = "Waiting for a phone".into();
            }
            let mut server = EspHttpServer::new(&Configuration {
                http_port: WIFI_TRANSFER_HTTP_PORT,
                stack_size: WIFI_TRANSFER_SERVER_STACK_BYTES,
                max_open_sockets: 2,
                max_sessions: 2,
                max_uri_handlers: 6,
                session_timeout: Duration::from_secs(60),
                ..Default::default()
            })?;
            server.fn_handler("/", Method::Get, move |request| {
                request
                    .into_ok_response()?
                    .write_all(SETUP_HTML.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;
            add_network_routes(
                &mut server,
                None,
                networks,
                &shared,
                Some(Arc::new(Mutex::new(scan))),
            )?;
            info!("rustmix-wave=wifi-setup status=ready url={url} ssid={ssid}");
            Ok(Self {
                _server: server,
                shared,
            })
        }

        #[must_use]
        pub fn snapshot(&self) -> WifiTransferSnapshot {
            lock(&self.shared).snapshot.clone()
        }

        #[must_use]
        pub fn is_expired(&self) -> bool {
            lock(&self.shared).last_activity.elapsed()
                >= Duration::from_secs(WIFI_TRANSFER_INACTIVITY_SECONDS)
        }
    }

    impl Drop for WifiTransferServer {
        fn drop(&mut self) {
            info!("rustmix-wave=wifi-transfer-server status=stopped");
        }
    }

    fn lock(shared: &Arc<Mutex<SharedStatus>>) -> std::sync::MutexGuard<'_, SharedStatus> {
        shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn authenticate(uri: &str, expected: &str) -> Result<()> {
        if query_value(uri, "code").as_deref() == Some(expected) {
            Ok(())
        } else {
            bail!("session code required")
        }
    }

    fn required_query(uri: &str, key: &str) -> Result<String> {
        query_value(uri, key).ok_or_else(|| anyhow!("missing query parameter: {key}"))
    }

    fn reject_protected(relative: &str) -> Result<()> {
        if is_protected_portal_path(relative) {
            bail!("protected configuration file")
        }
        Ok(())
    }

    fn sibling_path(path: &Path, extension: &str) -> Result<std::path::PathBuf> {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| anyhow!("filename required"))?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("parent folder required"))?;
        Ok(parent.join(format!("{stem}.{extension}")))
    }

    fn temporary_path(path: &Path) -> Result<std::path::PathBuf> {
        sibling_path(path, "TMP")
    }

    fn commit_atomic_upload(temporary: &Path, destination: &Path) -> Result<()> {
        let backup = sibling_path(destination, "BAK")?;
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        let had_destination = destination.exists();
        if had_destination {
            fs::rename(destination, &backup)?;
        }
        if let Err(error) = fs::rename(temporary, destination) {
            if had_destination {
                let _ = fs::rename(&backup, destination);
            }
            let _ = fs::remove_file(temporary);
            return Err(error.into());
        }
        if had_destination {
            fs::remove_file(&backup)?;
        }
        Ok(())
    }

    fn list_directory_json(relative: &str) -> Result<String> {
        let path = resolve_portal_path(relative).map_err(|error| anyhow!(error))?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)?.take(WIFI_TRANSFER_MAX_DIRECTORY_ROWS) {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let child_relative = format!("{}/{}", relative.trim_end_matches('/'), name);
            if is_protected_portal_path(&child_relative) || !super::is_portal_safe_name(&name) {
                continue;
            }
            let metadata = entry.metadata()?;
            entries.push((name, metadata.is_dir(), metadata.len()));
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let mut json = String::from("[");
        for (index, (name, directory, size)) in entries.into_iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            let kind = if directory { "folder" } else { "file" };
            json.push_str(&format!(
                r#"{{"name":"{}","kind":"{kind}","size":{size}}}"#,
                json_escape(&name)
            ));
        }
        json.push(']');
        Ok(json)
    }

    fn json_escape(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_portal_safe_name, is_protected_portal_path, query_value, resolve_portal_path,
        saved_json, scan_json, setup_credentials, WifiTransferSnapshot, WifiTransferState,
    };
    use std::path::PathBuf;

    use crate::json_lite::{self, JsonValue};
    use crate::wifi_networks::{NetworkBook, ScanEntry};

    /// Names a radio can report: quotes, backslashes, control characters,
    /// accents, CJK, emoji and 32 bytes.
    const AWKWARD_NAMES: [&str; 8] = [
        "Casa \"Fernández\"",
        "back\\slash\\",
        "tab\there\nline\u{1}\u{1f}",
        "Ñandú café",
        "咖啡店",
        "Wi-Fi 🦀",
        "\u{7f}del",
        "abcdefghijklmnopqrstuvwxyz012345",
    ];

    fn string_at<'a>(value: &'a JsonValue, path: &[&str]) -> &'a str {
        let mut current = value;
        for key in path {
            current = current.get(key).expect("key present");
        }
        current.as_str().expect("a string")
    }

    #[test]
    fn the_scan_list_is_json_that_reads_back_to_the_same_names() {
        let found: Vec<ScanEntry> = AWKWARD_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| ScanEntry {
                ssid: (*name).into(),
                rssi_dbm: -40 - index as i32,
                secured: index % 2 == 0,
            })
            .collect();
        let json = scan_json(&found);
        // Strict JSON has no raw control characters; json_lite would take them.
        assert!(!json.chars().any(|c| (c as u32) < 0x20));
        let parsed = json_lite::parse(&json).expect("valid JSON");
        for (index, entry) in found.iter().enumerate() {
            let row = parsed.index(index).expect("one row per network");
            assert_eq!(string_at(row, &["ssid"]), entry.ssid);
            assert_eq!(
                row.get("rssi").and_then(JsonValue::as_i64),
                Some(i64::from(entry.rssi_dbm))
            );
            assert_eq!(row.get("secured"), Some(&JsonValue::Bool(entry.secured)));
        }
        assert!(parsed.index(found.len()).is_none());
        assert_eq!(scan_json(&[]), "[]");
    }

    #[test]
    fn the_saved_list_is_json_that_reads_back_to_the_same_names() {
        let mut book = NetworkBook::default();
        for name in AWKWARD_NAMES.iter().take(6) {
            book.add(name, "password1").expect("a valid network");
        }
        let json = saved_json(&book);
        assert!(!json.contains("password1"), "names only, never passwords");
        assert!(!json.chars().any(|c| (c as u32) < 0x20));
        let parsed = json_lite::parse(&json).expect("valid JSON");
        assert_eq!(parsed.get("available"), Some(&JsonValue::Bool(true)));
        let saved = parsed.get("saved").expect("the saved list");
        for (index, network) in book.networks().iter().enumerate() {
            assert_eq!(
                saved.index(index).and_then(JsonValue::as_str),
                Some(network.ssid.as_str())
            );
        }
        assert!(saved.index(book.networks().len()).is_none());
        let empty = saved_json(&NetworkBook::default());
        let empty = json_lite::parse(&empty).expect("valid JSON");
        assert!(empty.get("saved").unwrap().index(0).is_none());
    }

    #[test]
    fn setup_credentials_keep_their_shape_at_the_extremes() {
        let extremes = [
            0,
            1,
            0xFFFF,
            0x1_0000,
            99_999_999,
            100_000_000,
            u32::MAX - 1,
            u32::MAX,
        ];
        let upper_hex = |c: char| c.is_ascii_digit() || ('A'..='F').contains(&c);
        for a in extremes {
            for b in extremes {
                let (name, password) = setup_credentials(a, b);
                let hex = name.strip_prefix("Wave-").expect("starts with Wave-");
                assert_eq!(hex.len(), 4, "{name}");
                assert!(hex.chars().all(upper_hex), "{name}");
                assert_eq!(password.len(), 8, "{password}");
                assert!(password.bytes().all(|d| d.is_ascii_digit()), "{password}");
            }
        }
        assert_eq!(
            setup_credentials(0x1234_3FA2, 7),
            ("Wave-3FA2".into(), "00000007".into())
        );
        assert_eq!(setup_credentials(u32::MAX, u32::MAX).0, "Wave-FFFF");
    }

    #[test]
    fn portal_is_off_until_the_user_explicitly_starts_it() {
        let snapshot = WifiTransferSnapshot::default();
        assert_eq!(snapshot.state, WifiTransferState::Off);
        assert!(!snapshot.is_active());
    }

    #[test]
    fn portal_paths_are_confined_and_fat_safe() {
        assert!(resolve_portal_path("/BOOKS/POIROT01.EPU").is_ok());
        assert!(resolve_portal_path("/BOOKS/Cien años de soledad.epub").is_ok());
        assert!(resolve_portal_path("../WIFI.TXT").is_err());
        assert!(resolve_portal_path("/BOOKS/a:b.txt").is_err());
        assert!(is_portal_safe_name("MAIN.LUA"));
        assert!(is_portal_safe_name("Poirot investiga.txt"));
        assert!(!is_portal_safe_name("a\\b.txt"));
        assert!(!is_portal_safe_name("book.txt."));
        assert!(!is_portal_safe_name("book.txt "));
    }

    #[test]
    fn paths_are_decoded_once() {
        let literal = resolve_portal_path("%57IFI.TXT").unwrap();
        assert_eq!(literal, PathBuf::from("/sdcard/RUSTMIX/%57IFI.TXT"));
    }

    #[test]
    fn configuration_files_are_protected() {
        assert!(is_protected_portal_path("/WIFI.TXT"));
        assert!(is_protected_portal_path("ALARMS.TXT"));
        assert!(is_protected_portal_path("/VOICE/META.TXT"));
        assert!(is_protected_portal_path("VOICE/SETTINGS.TXT"));
        assert!(is_protected_portal_path("APPS/CALENDAR/EVENTS.TMP"));
        assert!(is_protected_portal_path("APPS/CALENDAR/EVENTS.BAK"));
        assert!(!is_protected_portal_path("APPS/CALENDAR/EVENTS.TXT"));
        assert!(!is_protected_portal_path("/VOICE/VOICE001.WAV"));
        assert!(!is_protected_portal_path("/BOOKS/NOTES001.TXT"));
    }

    #[test]
    fn protection_survives_other_spellings() {
        assert!(is_protected_portal_path("./wifi.txt"));
        assert!(is_protected_portal_path("VOICE//Meta.txt"));
        assert!(is_protected_portal_path("WıFı.TXT"));
        assert!(resolve_portal_path("/./WIFI.TXT").is_err());
        assert!(resolve_portal_path("/VOICE//META.TXT").is_err());
    }

    #[test]
    fn query_parser_decodes_portal_paths() {
        assert_eq!(
            query_value("/api/list?code=123456&path=%2FBOOKS", "path").as_deref(),
            Some("/BOOKS")
        );
    }
}

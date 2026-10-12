//! XiaoZhi voice chat, ported from the xiaozhi-esp32 firmware: the OTA
//! check and linking, the conversation, device tools over MCP, and on the
//! device the call thread with its WebSocket and Opus audio.

pub mod activation;
pub mod call;
pub mod mcp;
pub mod protocol;

#[cfg(target_os = "espidf")]
pub mod espidf;

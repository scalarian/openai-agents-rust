# Realtime Transport

Use this page to choose the transport shape beneath a Rust realtime session.

The existing detailed Rust transport overview is [transports.md](transports.md). This singular `transport.md` page exists for upstream path parity and summarizes the decision points.

## Decision Guide

| Goal | Start with |
| --- | --- |
| Server-owned realtime model session | OpenAI realtime WebSocket model path |
| Telephony-oriented attach flow | OpenAI realtime SIP model path |
| Twilio Media Streams bridge | `realtime_twilio_media_stream.rs` |
| Twilio SIP bridge | `realtime_twilio_sip.rs` |
| Platform-specific edge bridge | Cloudflare realtime transport extensions |

## Guidance

- Use WebSocket when your service owns the live model session.
- Use SIP when your application is attaching agents to telephony-style calls.
- Use Twilio or Cloudflare adapters when your deployment already receives platform-specific transport events and needs to translate them into runtime session events.
- Browser WebRTC client code is outside this Rust runtime surface; keep browser capture/playback in your application and bridge server-side state intentionally.

## Runnable Examples

- [realtime_session.rs](../../crates/openai-agents/examples/realtime_session.rs)
- [realtime_twilio_media_stream.rs](../../crates/openai-agents/examples/realtime_twilio_media_stream.rs)
- [realtime_twilio_sip.rs](../../crates/openai-agents/examples/realtime_twilio_sip.rs)

## Read Next

- [transports.md](transports.md)
- [guide.md](guide.md)
- [events.md](events.md)
- [audio.md](audio.md)

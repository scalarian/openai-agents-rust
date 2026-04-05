# Realtime Transports

Use this page when you need to choose or understand the transport beneath a realtime session.

## Main Runtime Surfaces

- OpenAI realtime websocket model
- OpenAI realtime SIP model
- extensions transports such as Cloudflare and Twilio adapters

## Transport Guidance

- use websocket when you want direct model session traffic
- use SIP when your environment already speaks telephony-style session control
- use extension adapters when you need a bridge between a platform transport and the runtime session model

## Read Next

- [README.md](README.md)
- [../ref/realtime.md](../ref/realtime.md)

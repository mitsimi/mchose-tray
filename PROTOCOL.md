# K7 V2 Ultra+ battery read

Traced from the public [MCHOSE M HUB](https://www.mchose.com.cn/) bundle and
verified on this receiver in September 2026. This is MCHOSE's protocol, not
the ATK Nordic54 protocol, despite the shared MCU family.

| HID selector | Value |
| --- | --- |
| Product name | MCHOSE K7 V2 Ultra+ |
| VID:PID | `3837:1014` |
| Interface | `2` |
| Usage page:usage | `FF01:0001` |
| Transport | Interrupt write/read, 64 bytes including report ID |

Read-only device-info request, padded with zeros to 64 bytes:

```text
4d 01 01 00 00 09 00 00 08
```

Real reply, also zero-padded to 64 bytes:

```text
4d 01 01 10 00 09 00 00 37 38 26 40 04 00 00 00 00 10 02 00 53 00 08 e4 d8
```

Byte 0 is report ID `4D`; byte 1 is version `01`; byte 2 is flags `01`.
Byte 3 gives the payload length. Bytes 4–5 are the little-endian command
`0900`, byte 6 is the business code (zero), and byte 7 is a sequence value.
Payload starts at byte 8. The following byte is the XOR of bytes 2 through
the end of the payload; unused padding is not checksummed.

| Payload offset | Meaning |
| --- | --- |
| 0–1 | Mouse VID, little-endian (`3837`) |
| 2–3 | Mouse-reported product ID; distinct from receiver PID |
| 10 | Connection state (`02` accepted; zero is unavailable) |
| 11 | Charging flag (`00` observed; `01` is charging in M HUB) |
| 12 | Battery percentage (`53` hex = 83%) |

The reader accepts only a matching, complete, checksummed reply with a valid
percentage, charge flag and connection state. The sequence byte may vary but is
covered by the checksum. It ignores other reports and waits at most 800 ms
for an input reply. An absent receiver, I/O failure or timeout never produces
a cached percentage. Every poll re-enumerates and reopens the receiver to
allow recovery after reconnection.

The app only sends `0900`. No pairing, settings, reset, DPI or firmware command
is implemented. Charging and physical link-loss behavior are not yet verified
on this mouse; the app labels unavailable readings conservatively.

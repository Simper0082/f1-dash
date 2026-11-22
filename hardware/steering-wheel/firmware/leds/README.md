# Wheel LED firmware

Polls `/api/wheel?driver=<n>` once a second and drives a WS2812B strip from GPIO14.

| Priority | Condition | LEDs |
|---|---|---|
| 1 | Penalty (unserved, first 20 s) | red flash |
| 2 | Red flag | solid red |
| 3 | Followed driver in pit | pulse in team colour |
| 4 | Safety car / VSC | flashing yellow |
| 5 | Yellow flag | solid yellow |
| 6 | Green | dim team-colour glow |
| – | No data for 5 s | slow dim white breathing |

`src/led_logic.h` has no hardware dependencies. Test it on a PC:
`g++ -std=c++17 test/test_logic.cpp -o /tmp/t && /tmp/t`

Setup: copy `src/config.example.h` to `src/config.h` and fill in Wi-Fi, API address and LED count.
After the first USB flash, updates go over Wi-Fi (OTA, host name `wheel-leds`).

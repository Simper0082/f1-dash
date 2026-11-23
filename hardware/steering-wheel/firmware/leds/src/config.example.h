// Copy to config.h (git-ignored) and fill in.
#pragma once

#define WIFI_SSID "your-wifi"
#define WIFI_PASS "your-password"

// Base URL of the realtime service, no trailing slash
#define API_BASE "http://192.168.1.50:4000"

// Racing number to follow until the screen's driver picker drives this
#define DRIVER_NUMBER "3"

#define LED_PIN 14        // via the 74AHCT125 level shifter + 330 ohm
#define NUM_LEDS 14       // 14 LEDs behind the top light cut-out (15 fit, 14 leaves room)
#define MAX_BRIGHTNESS 64 // of 255, ~25 %. Keeps the 5 V draw and the heat inside the wheel low

#include <Arduino.h>
#include <ArduinoJson.h>
#include <ArduinoOTA.h>
#include <FastLED.h>
#include <HTTPClient.h>
#include <WiFi.h>

#include "config.h"
#include "led_logic.h"

static CRGB leds[NUM_LEDS];
static WheelState state;
static uint32_t lastOkMs = 0;
static uint32_t lastPollMs = 0;
static const char* lastWhy = "";

static Rgb parseTeam(const char* hex) {
  if (!hex || strlen(hex) != 6) return state.team;
  uint32_t v = strtoul(hex, nullptr, 16);
  return {(uint8_t)(v >> 16), (uint8_t)(v >> 8), (uint8_t)v};
}

static TrackKind parseTrack(const char* k) {
  if (!k) return TrackKind::Clear;
  if (!strcmp(k, "yellow")) return TrackKind::Yellow;
  if (!strcmp(k, "sc")) return TrackKind::Sc;
  if (!strcmp(k, "vsc")) return TrackKind::Vsc;
  if (!strcmp(k, "red")) return TrackKind::Red;
  return TrackKind::Clear;
}

static bool poll() {
  if (WiFi.status() != WL_CONNECTED) return false;
  HTTPClient http;
  http.setTimeout(1500);
  http.begin(String(API_BASE) + "/api/wheel?driver=" DRIVER_NUMBER);
  if (http.GET() != 200) { http.end(); return false; }

  // only keep the fields we need, the full reply is ~1 KB
  JsonDocument filter;
  filter["live"] = true;
  filter["driver"]["team_colour"] = true;
  filter["track"]["kind"] = true;
  filter["pit"]["in_pit"] = true;
  filter["penalty"]["served"] = true;
  filter["penalty"]["age_s"] = true;

  JsonDocument doc;
  DeserializationError err = deserializeJson(doc, http.getStream(), DeserializationOption::Filter(filter));
  http.end();
  if (err || !(doc["live"] | false)) return false;

  state.team = parseTeam(doc["driver"]["team_colour"] | (const char*)nullptr);
  state.track = parseTrack(doc["track"]["kind"] | (const char*)nullptr);
  state.inPit = doc["pit"]["in_pit"] | false;
  JsonObjectConst pen = doc["penalty"];
  state.penaltyActive = !pen.isNull() && !(pen["served"] | true) && (pen["age_s"] | 9999) < PENALTY_FLASH_S;
  return true;
}

void setup() {
  Serial.begin(115200);
  FastLED.addLeds<WS2812B, LED_PIN, GRB>(leds, NUM_LEDS);
  FastLED.setBrightness(MAX_BRIGHTNESS);
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASS);
  ArduinoOTA.setHostname("wheel-leds");
  ArduinoOTA.begin();
}

void loop() {
  ArduinoOTA.handle();
  uint32_t now = millis();

  if (now - lastPollMs >= 1000) {
    lastPollMs = now;
    if (poll()) lastOkMs = now;
  }
  // no good data for 5 s -> gentle offline breathing, never a stale alarm colour
  state.valid = lastOkMs != 0 && now - lastOkMs < 5000;

  Look look = chooseLook(state);
  if (look.why != lastWhy) { lastWhy = look.why; Serial.println(look.why); }

  Rgb c = frameColour(look, now);
  fill_solid(leds, NUM_LEDS, CRGB(c.r, c.g, c.b));
  FastLED.show();
  delay(10);
}

// Pure LED decision logic: no Arduino, no FastLED. Compiles on a PC so it can be unit tested.
#pragma once
#include <stdint.h>

struct Rgb { uint8_t r, g, b; };

enum class TrackKind : uint8_t { Clear, Yellow, Sc, Vsc, Red };

struct WheelState {
  bool valid = false;            // false until a poll succeeded recently
  TrackKind track = TrackKind::Clear;
  bool inPit = false;
  bool penaltyActive = false;    // unserved and younger than PENALTY_FLASH_S
  Rgb team = {70, 130, 220};
};

constexpr uint32_t PENALTY_FLASH_S = 20;

enum class Pattern : uint8_t { Solid, Flash, Pulse, Breathe };

struct Look {
  Rgb colour;
  Pattern pattern;
  const char* why;  // for the serial log and tests
};

constexpr Rgb YELLOW = {255, 170, 0};
constexpr Rgb RED = {255, 0, 0};

inline Rgb scale(Rgb c, uint8_t k) {
  return {(uint8_t)((c.r * k) / 255), (uint8_t)((c.g * k) / 255), (uint8_t)((c.b * k) / 255)};
}

// Priority, highest first: penalty, red flag, in pit, SC/VSC, yellow, green glow, offline.
inline Look chooseLook(const WheelState& s) {
  if (!s.valid) return {{60, 60, 60}, Pattern::Breathe, "offline"};
  if (s.penaltyActive) return {RED, Pattern::Flash, "penalty"};
  if (s.track == TrackKind::Red) return {RED, Pattern::Solid, "red flag"};
  if (s.inPit) return {s.team, Pattern::Pulse, "in pit"};
  if (s.track == TrackKind::Sc || s.track == TrackKind::Vsc) return {YELLOW, Pattern::Flash, "sc/vsc"};
  if (s.track == TrackKind::Yellow) return {YELLOW, Pattern::Solid, "yellow"};
  return {scale(s.team, 40), Pattern::Solid, "green"};
}

// 0..255 brightness multiplier for a pattern at time nowMs
inline uint8_t envelope(Pattern p, uint32_t nowMs) {
  switch (p) {
    case Pattern::Solid: return 255;
    case Pattern::Flash: return ((nowMs / 250) % 2 == 0) ? 255 : 0;  // 2 Hz
    case Pattern::Pulse: {                                            // triangle, 1.2 s
      uint32_t t = nowMs % 1200;
      uint32_t up = t < 600 ? t : 1200 - t;
      return (uint8_t)(40 + (up * 215) / 600);
    }
    case Pattern::Breathe: {                                          // slower, shallower, 4 s
      uint32_t t = nowMs % 4000;
      uint32_t up = t < 2000 ? t : 4000 - t;
      return (uint8_t)(20 + (up * 80) / 2000);
    }
  }
  return 255;
}

inline Rgb frameColour(const Look& l, uint32_t nowMs) { return scale(l.colour, envelope(l.pattern, nowMs)); }

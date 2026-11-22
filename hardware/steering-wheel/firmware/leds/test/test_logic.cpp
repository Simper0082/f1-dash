#include <cassert>
#include <cstdio>
#include <cstring>
#include "../src/led_logic.h"

static WheelState base() { WheelState s; s.valid = true; s.team = {0x47, 0x81, 0xD7}; return s; }

int main() {
  WheelState s = base();
  assert(!strcmp(chooseLook(s).why, "green"));

  s.track = TrackKind::Yellow; assert(!strcmp(chooseLook(s).why, "yellow"));
  s.track = TrackKind::Sc;     assert(!strcmp(chooseLook(s).why, "sc/vsc"));
  s.track = TrackKind::Vsc;    assert(chooseLook(s).pattern == Pattern::Flash);
  s.track = TrackKind::Red;    assert(!strcmp(chooseLook(s).why, "red flag"));

  // pit beats SC/VSC/yellow, uses team colour
  s = base(); s.track = TrackKind::Sc; s.inPit = true;
  Look l = chooseLook(s);
  assert(!strcmp(l.why, "in pit") && l.colour.r == 0x47 && l.pattern == Pattern::Pulse);

  // penalty beats everything
  s.penaltyActive = true; s.track = TrackKind::Red;
  assert(!strcmp(chooseLook(s).why, "penalty"));

  // offline never shows an alarm
  s.valid = false; assert(!strcmp(chooseLook(s).why, "offline"));

  // flash toggles at 2 Hz, pulse stays within bounds
  assert(envelope(Pattern::Flash, 0) == 255 && envelope(Pattern::Flash, 250) == 0);
  for (uint32_t t = 0; t < 2400; t += 7) { uint8_t e = envelope(Pattern::Pulse, t); assert(e >= 40); }
  printf("led logic ok\n");
  return 0;
}

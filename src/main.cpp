#include "esp32-hal-ledc.h"
#include "Arduino.h"
#include "esp32-hal.h"
#include <stdarg.h>

// blessed be https://arduino.stackexchange.com/questions/56517/formatting-strings-in-arduino-for-output
bool debugging_enabled = 0;
void dbgln(const char* input...) {
	if (!debugging_enabled)return;
	va_list args;
	va_start(args, input);
	for(const char* i=input; *i!=0; ++i) {
		if(*i!='%') { Serial.print(*i); continue; }
		switch(*(++i)) {
		case '%': Serial.print('%'); break;
		case 's': Serial.print(va_arg(args, char*)); break;
		case 'd': Serial.print(va_arg(args, int), DEC); break;
		case 'b': Serial.print(va_arg(args, int), BIN); break;
		case 'o': Serial.print(va_arg(args, int), OCT); break;
		case 'x': Serial.print(va_arg(args, int), HEX); break;
		case 'f': Serial.print(va_arg(args, double), 2); break;
		}
	}
	Serial.println();
	va_end(args);
}

struct ledState {
	uint16_t dwarm;
	uint16_t dcold;
	uint16_t cwarm;
	uint16_t ccold;
} typedef ledState;

uint8_t relay_pin = 25;
// desk
uint8_t dwarm_pin = 33;   // -WW
uint8_t dcold_pin = 32;   // -CW
// ceiling
uint8_t cwarm_pin = 26;   // -WW
uint8_t ccold_pin = 27;   // -CW
// interpolate anim
ledState currentState;
bool animValid = false;
uint32_t animStart;
ledState animStartState = {0,0,0,0};
uint32_t animLength;
ledState animEndState = {0,0,0,0};

const float TAU = PI * 2;
const uint16_t U16_MAX = 65535;
const uint16_t ANALOG_MAX = 4095; // 12 bit

void setup()
{
	Serial.begin(115200);
	delay(10);
	pinMode(relay_pin, OUTPUT);
	ledcAttachPin(dwarm_pin, 1);
	ledcAttachPin(dcold_pin, 2);
	ledcAttachPin(cwarm_pin, 3);
	ledcAttachPin(ccold_pin, 4);
	// it doesn't work at 3kHz, go figure
	ledcSetup(1, 1200, 16);
	ledcSetup(2, 1200, 16);
	ledcSetup(3, 1200, 16);
	ledcSetup(4, 1200, 16);
	ledcWrite(1, 0);
	ledcWrite(2, 0);
	ledcWrite(3, 0);
	ledcWrite(4, 0);
}

enum instr {
	IIdentify = 0,
	IImmediate = 1,
	IInterpolateFrame = 2,
	IDebugEnable = 3,
	INoInterpolate = 4,
	IRelayControl = 5,
} typedef instr;

inline uint16_t read_u16() {
	while (Serial.available() < 2) {}
	uint16_t ret;
	ret = Serial.read() << 8;
	ret |= Serial.read();
	return ret;
}

inline uint32_t read_u32() {
	while (Serial.available() < 4) {}
	uint32_t ret;
	ret = Serial.read() << 24;
	ret |= Serial.read() << 16;
	ret |= Serial.read() << 8;
	ret |= Serial.read();
	return ret;
}

void read_led_state(ledState *out) {
	out->dwarm = read_u16();
	out->dcold = read_u16();
	out->cwarm = read_u16();
	out->ccold = read_u16();
	dbgln("++ read_led_state (%d,%d|%d,%d) ++", out->cwarm, out->ccold, out->dwarm, out->dcold);
}

void set_led_state(ledState *in) {
	dbgln("++ set_led_state (%d,%d|%d,%d) ++", in->cwarm, in->ccold, in->dwarm, in->dcold);
	ledcWrite(1, in->dwarm);
	ledcWrite(2, in->dcold);
	ledcWrite(3, in->cwarm);
	ledcWrite(4, in->ccold);
}

void handle_command()
{
	if (Serial.available() > 0) {
		instr inst = (instr)Serial.read();
		if (inst == IIdentify) {
			Serial.println("desk-fcobs");
			return;
		} else if (inst == IInterpolateFrame) {
			dbgln("IInterpolate");
			// 02 01808580 001b7740 ffff
			// 0201808580001b7740ffffffffffffffff
			// 02000001f4000003e8ffffffffffffffff
			//
			// few seconds, full warm white
			// 02 0000000f 00000fff ffff0000ffff0000
			// typ start  | length |  end led state
			// 02 01700ac0 01700ac0 0000ffff0000ffff
			// in 6.7 hours, cold white fade over 30 min
			// 02 01650e40 01700ac0 0000ffff0000ffff
			// in 6.5 hours, cold white fade over 30 min
			// 02 018603c0 01700ac0 0000ffff0000ffff
			// in ~7.1 hours, cold white fade over 30 min
			// 02 019bfcc0 0001d4c0 0000ffff0000ffff
			// in ~7.5 hours, cold white fade over 2 min
			//
			// few seconds, fade out turn it all off; FIXME: fades the wrong direction then turns off correctly at end
			// 02 0000000f 00000fff ffffffffffffffff
			animValid = true;
			animStart = millis() + read_u32();
			animLength = read_u32();
			read_led_state(&animEndState);
		} else if (inst == IImmediate) {
			// all off
			// 01ffffffffffffffff
			dbgln("IImmediate");
			read_led_state(&currentState);
			memcpy(&animStartState, &currentState, sizeof(ledState));
			set_led_state(&currentState);
			dbgln("OK");
		} else if (inst == IDebugEnable) {
			debugging_enabled = true;
		} else if (inst == INoInterpolate) {
			animValid = false;
		} else if (inst == IRelayControl) {
			digitalWrite(relay_pin, Serial.read() == 0 ? LOW : HIGH);
		} else {
			dbgln("I??");
		}
	}
}

void interpolate() {
	if (!animValid || millis() < animStart)
		return;
	dbgln("animValid %d: animStart=%d,animLength=%d,now=%d", animValid,animStart,animLength,millis());
	uint32_t end = animStart + animLength;
	double progress = ((double)millis() - (double)end) / (double)animLength / 2 + 0.5;
	if (progress<0.0) progress=0;
	if (progress>1.0) progress=1.0;
	dbgln("progress=%f,end=%d", progress,end);
	ledState work = {};
	uint16_t *astart = (uint16_t*)&animStartState;
	uint16_t *aend = (uint16_t*)&animEndState;
	uint16_t *awork = (uint16_t*)&work;
	for (int i = 0; i < 4; i++) {
		dbgln("iter#%d: aenp=%f, astrp=%f(%d * %f)", i, aend[i]*progress,astart[i]*(1.0-progress),astart[i],1.0-progress);
		// FIXME: all these inverts are to avoid touching the surrounding code,
		// but still get some reasonable units
		// astart[i] ~= U16_MAX;
		// aend[i] ~= U16_MAX;

		// lerp
		awork[i] = (uint16_t)((1.-progress)*((double)(astart[i])) + progress*((double)(aend[1])));

		// astart[i] ~= U16_MAX;
		// aend[i] ~= U16_MAX;
		// awork[i] ~= U16_MAX;
	}

	set_led_state(&work);

	if (progress >= 1.0) {
		animValid = false;
		return;
	}
}

void loop() {
	handle_command();
	interpolate();
}

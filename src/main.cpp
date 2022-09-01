#include "esp32-hal-ledc.h"
#include "Arduino.h"
#include "esp32-hal.h"

// desk
uint8_t dwarm_pin = 33;   // -WW
uint8_t dcold_pin = 32;   // -CW
// ceiling
uint8_t cwarm_pin = 26;   // -WW
uint8_t ccold_pin = 27;   // -CW

const float TAU = PI * 2;
const uint16_t U16_MAX = 65535;
const uint16_t ANALOG_MAX = 4095; // 12 bit

void setup()
{
	Serial.begin(115200);
	delay(10);
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

void loop()
{
	if (Serial.available() >= 4) {
		uint16_t dwarm, dcold, cwarm, ccold;
		dwarm = Serial.read() << 8; dwarm |= Serial.read();
		dcold = Serial.read() << 8; dcold |= Serial.read();
		cwarm = Serial.read() << 8; cwarm |= Serial.read();
		ccold = Serial.read() << 8; ccold |= Serial.read();
		Serial.println("++");
		Serial.println(cwarm);
		Serial.println(ccold);
		Serial.println("+-+");
		Serial.println(dwarm);
		Serial.println(dcold);
		Serial.println("--");
		ledcWrite(1, dwarm);
		ledcWrite(2, dcold);
		ledcWrite(3, cwarm);
		ledcWrite(4, ccold);
		Serial.println("OK");
	}
}

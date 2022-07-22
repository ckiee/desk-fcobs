#include "esp32-hal-ledc.h"
#include "Arduino.h"
#include "esp32-hal.h"

uint8_t warm_pin = 33;   // -WW
uint8_t cold_pin = 32;   // -CW
uint8_t supply_pin = 26; // +24V


const float TAU = PI * 2;
const uint16_t U16_MAX = 65535;
const uint16_t ANALOG_MAX = 4095; // 12 bit

void setup()
{
	Serial.begin(115200);
	delay(10);
	ledcAttachPin(warm_pin, 1);
	ledcAttachPin(cold_pin, 2);
	ledcAttachPin(supply_pin, 3);
	// TODO: change freq to >3kHz
	ledcSetup(1, 1200, 16);
	ledcSetup(2, 1200, 16);
	ledcSetup(3, 1200, 16);
	ledcWrite(1, 0);
	ledcWrite(2, 0);
	ledcWrite(3, U16_MAX);
}

void loop()
{
	if (Serial.available() >= 4) {
		uint16_t supply_val, warm_val, cold_val;
		supply_val = Serial.read() << 8;
		supply_val |= Serial.read();
		warm_val = Serial.read() << 8;
		warm_val |= Serial.read();
		cold_val = Serial.read() << 8;
		cold_val |= Serial.read();
		Serial.println("++");
		Serial.println(supply_val);
		Serial.println("+-+");
		Serial.println(warm_val);
		Serial.println(cold_val);
		Serial.println("--");
		if (warm_val > supply_val) warm_val = 0;
		if (cold_val > supply_val) cold_val = 0;
		ledcWrite(1, warm_val);
		ledcWrite(2, cold_val);
		ledcWrite(3, supply_val);
		Serial.println("OK");
	}
}

const int POT_COUNT = 6;
const int analogIndex[POT_COUNT] = {A0, A1, A2, A3, A9, A8};

int analogIndexValue[POT_COUNT];
int lastAnalogValue[POT_COUNT];

const int threshold = 10;

unsigned long lastHeartbeat = 0;
const unsigned long heartbeatInterval = 60000;

const int MUTE_PIN = A6;
const int MUTE_THRESHOLD = 1000;      // "almost full" analog signal
bool invertMute = false;
bool lastMuteState = false;

bool muteStableState = false;
int muteDebounceCounter = 0;
const int MUTE_DEBOUNCE_COUNT = 3;      // ~90 ms at the existing 30 ms loop delay

void setup() {
  Serial.begin(9600);
  for (int i = 0; i < POT_COUNT; i++) {
    lastAnalogValue[i] = analogRead(analogIndex[i]);
  }

  muteStableState = (analogRead(MUTE_PIN) > MUTE_THRESHOLD);
  lastMuteState = muteStableState ^ invertMute;
}

void loop() {
  for (int i = 0; i < POT_COUNT; i++) {
    analogIndexValue[i] = analogRead(analogIndex[i]);
    if (abs(analogIndexValue[i] - lastAnalogValue[i]) > threshold) {
      String label = "A" + String(analogIndex[i] - A0);
      Serial.print(label);
      Serial.print("_");
      Serial.println(analogIndexValue[i]);
      lastAnalogValue[i] = analogIndexValue[i];
    }
  }

  bool rawMute = analogRead(MUTE_PIN) > MUTE_THRESHOLD;

  // simple debounce filter
  if (rawMute != muteStableState) {
    muteDebounceCounter++;
    if (muteDebounceCounter >= MUTE_DEBOUNCE_COUNT) {
      muteStableState = rawMute;
      muteDebounceCounter = 0;
    }
  } else {
    muteDebounceCounter = 0;
  }

  bool currentMuteState = muteStableState ^ invertMute;
  if (currentMuteState != lastMuteState) {
    Serial.println(currentMuteState ? "MUTE_ON" : "MUTE_OFF");
    lastMuteState = currentMuteState;
  }

  // heartbeat
  if (millis() - lastHeartbeat >= heartbeatInterval) {
    Serial.println("heartbeat");
    lastHeartbeat = millis();
  }

  delay(30);
}

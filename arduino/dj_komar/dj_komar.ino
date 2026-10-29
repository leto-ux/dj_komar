const int MAX_POTS = 12;
int potCount = 6;
int analogIndex[MAX_POTS] = {A0, A1, A2, A3, A9, A8};

int analogIndexValue[MAX_POTS];
int lastAnalogValue[MAX_POTS];

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

void initPots() {
  for (int i = 0; i < potCount; i++) {
    lastAnalogValue[i] = analogRead(analogIndex[i]);
  }
}

void parsePinsCommand(String cmd) {
  // Expected format: PINS:0,1,2,3,9,8
  String pinsStr = cmd.substring(5);
  pinsStr.trim();

  if (pinsStr.length() == 0) {
    potCount = 0;
    return;
  }

  int count = 0;
  int startIdx = 0;
  while (startIdx < (int)pinsStr.length() && count < MAX_POTS) {
    int commaIdx = pinsStr.indexOf(',', startIdx);
    String token;
    if (commaIdx == -1) {
      token = pinsStr.substring(startIdx);
      startIdx = pinsStr.length();
    } else {
      token = pinsStr.substring(startIdx, commaIdx);
      startIdx = commaIdx + 1;
    }
    token.trim();
    if (token.length() > 0) {
      int pinOffset = token.toInt();
      analogIndex[count] = A0 + pinOffset;
      count++;
    }
  }

  potCount = count;
  initPots();
}

void setup() {
  Serial.begin(9600);
  initPots();

  muteStableState = (analogRead(MUTE_PIN) > MUTE_THRESHOLD);
  lastMuteState = muteStableState ^ invertMute;
}

void loop() {
  if (Serial.available() > 0) {
    String cmd = Serial.readStringUntil('\n');
    cmd.trim();
    if (cmd.startsWith("PINS:")) {
      parsePinsCommand(cmd);
    }
  }

  for (int i = 0; i < potCount; i++) {
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

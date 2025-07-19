const int POT_COUNT = 6;
const int analogIndex[POT_COUNT] = {A0, A1, A2, A3, A4, A5};

int analogIndexValue[POT_COUNT];
int lastAnalogValue[POT_COUNT];
const int threshold = 10;

void setup() {
  Serial.begin(9600);
  for (int i = 0; i < POT_COUNT; i++) {
    lastAnalogValue[i] = analogRead(analogIndex[i]);
  }
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
  delay(30);
}

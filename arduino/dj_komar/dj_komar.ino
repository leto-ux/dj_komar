const int POT_COUNT = 9;
const int analogIndex[POT_COUNT] = {A0, A1, A2, A3, A4, A5, A6, A7};

int analogIndexValue[POT_COUNT];

void setup() {
  // put your setup code here, to run once:
  Serial.begin(9600);
}

void loop() {
  analogIndexValue[0] = analogRead(analogIndex[0]);
  Serial.println(analogIndexValue[0]);
  delay(300);
}

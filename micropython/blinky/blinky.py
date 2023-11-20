from machine import Pin
from utime import sleep_us

pin = Pin("LED", Pin.OUT)

while True:
   pin.toggle()
   sleep_us(100000)

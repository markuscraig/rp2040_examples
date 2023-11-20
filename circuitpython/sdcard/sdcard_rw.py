import board
import time
import busio
import sdcardio
import storage
import microcontroller

MOSI = board.GP11
MISO = board.GP12
clk = board.GP10
cs = board.GP15

print("Config SPI")
spi = busio.SPI(clk, MOSI=MOSI, MISO=MISO)

print("Create SDCard instance")
sd = sdcardio.SDCard(spi, cs)

print("Mounting fat vfs")
vfs = storage.VfsFat(sd)
storage.mount(vfs, '/sd')

print("Getting temperature")
temp = microcontroller.cpu.temperature

print("Opening pico.txt file for writing")
with open("/sd/pico.txt", "w") as file:
    file.write("opened pico.txt to write\r\n")

print("Appending pico.txt file")
with open("/sd/pico.txt", "a") as file:
    file.write("appending to pico.txt\r\n")

print("Appending pico.txt file again")
with open("/sd/pico.txt", "a") as file:
    file.write('temperature is {0:f} C\n'.format(temp))

print("Reading pico.txt file")
with open("/sd/pico.txt", "r") as file:
    print("Reading from pico.txt:")
    for line in file:
        print(line, end='')

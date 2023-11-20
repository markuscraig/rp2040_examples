import machine
import sdcard
import uos

# assign chip-select CS pin and start high
cs = machine.Pin(9, machine.Pin.OUT)

# init SPI peripheral; start with 1 MHz
spi = machine.SPI(1,
                  baudrate=1000000,
                  polarity=0,
                  phase=0,
                  bits=8,
                  firstbit=machine.SPI.MSB,
                  sck=machine.Pin(10),
                  mosi=machine.Pin(11),
                  miso=machine.Pin(8))

# init sd-card
sd = sdcard.SDCard(spi, cs)

# mount file-system
vfs = uos.VfsFat(sd)
uos.mount(vfs, "/sd")

# create a file and write data to  it
with open("/sd/test.txt", "w") as file:
   file.write("howdy...\r\n")
   file.write("just a quick test\r\n")

with open("/sd/test.txt", "r") as file:
   data = file.read()
   print(data)
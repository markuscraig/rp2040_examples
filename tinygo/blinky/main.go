package main

import (
	"machine"
	"time"
)

func main() {
	led := machine.LED
	led.Configure(machine.PinConfig{
		Mode: machine.PinOutput,
	})

	for {
		led.High()
		time.Sleep(time.Second)
		led.Low()
		time.Sleep(time.Second)
	}
}

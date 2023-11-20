//go:build pico

package main

import (
	"machine"
)

func init() {
	spi = machine.SPI1
	sckPin = machine.GPIO10
	sdoPin = machine.GPIO12
	sdiPin = machine.GPI11
	csPin = machine.GPI15

	ledPin = machine.LED
}

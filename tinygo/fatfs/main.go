//go:build tinygo
// +build tinygo

package main

import (
	"machine"
	"time"

	"tinygo.org/x/drivers/flash"
	"tinygo.org/x/tinyfs/examples/console"
	"tinygo.org/x/tinyfs/fatfs"
)

var (
	blockDevice = flash.NewSPI(
		machine.SPI1,
		machine.GPIO12, // SPI1_SDO_PIN
		machine.GPIO11, // SPI1_SDI_PIN
		machine.GPIO10, // SPI1_SCK_PIN
		machine.GPIO15, // SPI1_CS_PIN
	)

	filesystem = fatfs.New(blockDevice)
)

func main() {
	// Configure the flash device using the default auto-identifier function
	print("Configuring block device")
	config := &flash.DeviceConfig{Identifier: flash.DefaultDeviceIdentifier}
	if err := blockDevice.Configure(config); err != nil {
		for {
			time.Sleep(5 * time.Second)
			println("Config was not valid: "+err.Error(), "\r")
		}
	}

	// Configure FATFS with sector size (must match value in ff.h - use 512)
	print("Configuring fatfs")
	filesystem.Configure(&fatfs.Config{
		SectorSize: 512,
	})

	print("Starting console")
	console.RunFor(blockDevice, filesystem)
}

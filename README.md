# Axxon RS232 OX16PCI954 flash tool

Use at your own risk!

Coded for the following PCIe devices:
- https://axxon.io/lf781kb_pcie_8s_rs232_adapter/
- https://axxon.io/lf729kb_16s_rs232_pcie_adapter_card/

They identify as a "PCI Express-to-PCI Bridge" ("PEX 8112") and two or four instances of a
"OX16PCI954" PCI device (each providing four UART interfaces).

## Running

You'll need [`cargo` from rust](https://www.rust-lang.org/) to build the tool.

To build (if outdated / not compiled yet) and run:

    cargo run --bin axxon-ox16pci954-flash

To pass options to the program, insert `--` before them:

    cargo run --bin axxon-ox16pci954-flash -- --help

To actually flash the contained images (instead of just checking them) use:

    cargo run --bin axxon-ox16pci954-flash -- --flash

Before flashing the tool makes sure that the "PEX 8112" bridge contains `axxon` in the image at the
required place, and that "OX16PCI954" devices are on a bus behind such bridges.

## Hardware

### OX16PCI954

The chip supports various modes via two functions, which could be configured via two pins (although
there isn't a "good" way to only enable 4x UART on function 0 and disable function 1).

Only function 0 (4x UART) is of interest, function 1 is not wired.

Uses an (optional, but present on the Axxon cards) external EEPROM for configuration; the EEPROM
data is a word (16-bit) stream.

The flashing tool from the vendor disables function 1 by writing `0x00` to the lower 8-bit of the
device id, effectively making it `1415:9500` (which is the official ID for "disabled function 0",
not "disabled function 1", which would be `1415:9510`).

### PEX 8112

This is the PCIe-to-PCI bridge, and identifies as a device with a single function (PCI id
`10b5:8112`).

Documentation ("Data Book") was available from:
https://www.broadcom.com/products/pcie-switches-bridges/pcie-bridges/pex8112#documentation

Uses an (optional, but present on the Axxon cards) external EEPROM for configuration; the EEPROM
data is a byte stream.  At offset `0x78` (120) the data should contain the string `axxon` (although
the PCI device won't care about that).

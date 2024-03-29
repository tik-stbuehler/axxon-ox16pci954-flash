/*
Image main properties:
- disable Power Management Capability
- set Device-Specific Control to 0 (should be default though?)
- set Vendor and Device (default values again)

Decoded flash data:

- 0x003C = 60 bytes for configs (10 registers):
	@0x0010: 0x00000000 -- BAR0: Locate anywhere in 32-bit
	@0x0000: 0x811210B5 -- Vendor 10B5, Device 8112 (default)
	@0x0064: 0x00000020 -- Device Capability: Enable "Support 8-bit Tag" field
	@0x0100: 0x00010004 -- Power Budget Enhanced Capability Header (default)
	@0x100C: 0x03FEFE00 -- PCI Control:
		- PCI-To-PCI Express Retry Count set to 0xFE (default: 0x80)
		- PCI Express-to-PCI Retry Cound set to 0xFE (default: 0x00)
	@0x1020: 0x000010F0 -- GPIO Control
		- GPIO[1-3] Output enable (GPIO[0] is Output enabled by default)
		- GPIO Diagnostic Select: 10b (default: 01b)
	@0x1000: 0x00000033 -- Device Initialization (default)
	@0x0070: 0x00110000 -- Link control: default
	@0x0048: 0x00000000 -- Device-Specific Control (default 0)
	@0x0034: 0x00000050 -- PCI Capability pointer (default 0x40)
		- Skips (disables) Power Management Capability
		- Remaining: MSI and PCI Express
- 0x0004 bytes for shared memory
	0x55, 0x66, 0x77, 0x88

*/
pub static IMAGE: [u8; 0x46] = [
	0x5A, 0x03, 0x3C, 0x00, 0x10, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0xB5, 0x10, 0x12, 0x81,
	0x64, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x01,
	0x04, 0x00, 0x01, 0x00, 0x0C, 0x10, 0x00, 0xFE,
	0xFE, 0x03, 0x20, 0x10, 0xF0, 0x10, 0x00, 0x00,
	0x00, 0x10, 0x33, 0x00, 0x00, 0x00, 0x70, 0x00,
	0x00, 0x00, 0x11, 0x00, 0x48, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x34, 0x00, 0x50, 0x00, 0x00, 0x00,
	0x04, 0x00, 0x55, 0x66, 0x77, 0x88,
];

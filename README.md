[![CERN License](https://img.shields.io/badge/License-CERN%20OHL--W--V2-blue)](hardware/license/cern_ohl_w_v2.txt) [![License: BSD 2-Clause](https://img.shields.io/badge/License-BSD%202--Clause-blue)](software/LICENSE)
### Description
VeriTAS is a combination of a replay device for performing Tool-Assisted-Speedruns (aka Tool-Assisted-Superruns) on physical hardware, and software tooling that interfaces with the device and assists in other TAS replay tasks.

The RP2040 microcontroller is the brains of this device. This project is still in early development, but is intenteded as a replacement/continuation of my previous device the PICTAS.

### Software Tooling
The [VeriTAS software](software/README.md) is a Rust CLI tool that can perform various TAS replay related tasks, such as automated TAS dumping and video transcoding. It is also used for interfacing with the VeriTAS hardware.

### Input Displays
_Planned_

### Discord/Support
If you have questions or suggestions, you can find me on the [TASBot Labs](https://discord.tas.bot/) or the [TASVideos](https://discord.gg/7KSr7eZVzG) Discord servers.

### Licensing
The `/hardware/` is covered by the CERN-OHL-W-V2 license, while the `/firmware/` and `/software/` are covered by the BSD 2-Clause license.
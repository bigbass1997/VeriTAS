### Design Structure
The firmware is organized by game system. Due to the dual-core embedded environment provided by the RP2040,
each system has a module containing various functions that control the operating state of the device.
Note that systems do not use a struct to hold this state, but rather use module-level functions and static
variables.

While this is uncommon for Rust projects, in this situation, it is prefered. There is no reason to have
multiple instances of any given system, and it eases multi-core tasks such as refilling input buffers. It
also elliminates a need to pass around mutable references to the state.

---

#### GPIO Configuration
The RP2040 HAL is extremely dependent on types and traits to prevent improper use of pins at compile-time.
However, for some situations, like when a device needs to transition an already configured pin into a
different function at runtime, this design paradigm causes significant usability issues. Some of these
issues are highlighted on Github ([rp-rs/rp-hal #368](https://github.com/rp-rs/rp-hal/issues/368)).

Because of this, the VeriTAS firmware uses custom, low-level, GPIO functions. While this is unsafe, it
offers significantly better usability.

---

#### Communication Protocol
All transactions are initiated by the host computer, using a command-response protocol. Each transaction is
made up of a 4-byte (big-endian) length which notes how large the following payload is. The payload is
encoded/decoded using the `bincode` [spec](https://github.com/bincode-org/bincode/blob/trunk/docs/spec.md).
The decoded data can either be a command or response, depending on context. The host always initiates with
1 command, and expects 1 response. In turn, the device waits for 1 command, and returns 1 response.

Check [comms.rs](src/utilcore/comms.rs#L18-L39) for the available commands and responses.

_(notice: this protocol may change at any time during development)_

---

### Program Flow
Below are general descriptions of the states the device can be in. There are some tasks handled by the
secondary CPU core, that could be running regardless of the overall state of the device, such as sending
data to connected input displays or handling USB commands (which can touch many different parts of
the device).

#### Initial State
Upon boot, the device's basic peripherals/interrupts/etc are set up.

#### Idle State
After the device has been configured, or whenever a replay is stopped, the device will run in this state.
It can perform tasks like reprogramming input data, transition to a replay state, or other miscelaneous
tasks.

#### Replay State
The primary use of this device: input replay. Typically this state will be used to replay a TAS movie on
a specific system. However, it can also be used to relay manual controls from a host computer, to the
game system.
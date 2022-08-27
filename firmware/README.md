### Design Structure
The firmware is organized by game system. Due to the dual-core embedded environment provided by the RP2040,
each system has a module containing various functions that control the operating state of the device.
Note that systems do not use a struct to hold this state, but rather use module-level functions and static
variables.

While this is uncommon for Rust projects, in this situation, it is prefered. There is no reason to have
multiple instances of any given system, and it eases multi-core tasks such as refilling input buffers. It
also elliminates a need to pass around mutable references to the state.

Any mutable static variables should be multi-core safe, unless otherwise noted.

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
All transactions are initiated by the host computer, using a command-response protocol. All commands are
1 byte wide, and may be followed by additional data. Given the expected data for the command given,
the device should always respond with a 1-byte wide response, which may also be followed by additional data.

If any invalid/unrecognized commands are recieved, the device will return an `Err` response. Any extra data
sent by the host may cause unexpected behavior!

_(notice: this protocol may change at any time during development)_

##### _Commands:_
###### 0x01 - SetVeritasMode
Changes the device's mode. Command byte must be followed by a 1-byte mode ID.

| Mode | ID |
|------|----|
| Initial | 0x00 |
| Idle | 0x01 |
| ReplayN64 | 0x02 |
| ReplayNes | 0x03 |
| ReplayA2600 | 0x04 |
| ReplayGenesis | 0x05 |

_Invalid ID values will be interpreted as `Idle`._

###### 0x02 - ProvideInput
Enqueues input data for a specific system. Command byte must be followed by a 1-byte system ID, and then
the expected input data for that system.

| System | ID | Bytes |
|--------|----|-------|
| NES | 0x01 | 2 |
| N64 | 0x03 | 16 |
| Genesis | 0x08 | ? |
| A2600 | 0x09 | ? |

_Invalid ID values will cause an `Err` response._

###### 0x03 - GetStatus
_not implemented_

Requests the current status of the device, in the form of a `Text` response.

###### 0xAA - Ping
Ping! Host should recieve a `Pong` response.

##### _Responses:_
| Name | ID   |
|----|------|
| Ok | 0x01 |
| Text | 0x02 |
| BufferFull | 0xF0 |
| Pong | 0x55 |
| Err | 0x00 |

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
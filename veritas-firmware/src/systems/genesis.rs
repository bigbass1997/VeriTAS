use cortex_m::asm::nop;
use cortex_m::delay::Delay;
use defmt::{info, warn};
use heapless::spsc::Queue;
use rp2040_pac::Interrupt::{IO_IRQ_BANK0, TIMER_IRQ_0};
use rp2040_pac::{IO_BANK0, SIO};
use crate::hal::gpio::{Gpio, PIN_CNT_1, PIN_CNT_10, PIN_CNT_11, PIN_CNT_12, PIN_CNT_13, PIN_CNT_14, PIN_CNT_16, PIN_CNT_2, PIN_CNT_3, PIN_CNT_4, PIN_CNT_5, PIN_CNT_6, PIN_CNT_7, PIN_CNT_9, PIN_DETECT};
use crate::hal::interrupts;
use crate::hal::interrupts::Edge;
use crate::replaycore::{REPLAY_STATE, VERITAS_MODE, VeritasMode};
use crate::utilcore::displays;
use crate::utilcore::displays::Port;
use crate::VTABLE0;

pub static mut INPUT_BUFFER: Queue<[u8; 4], 1024> = Queue::new();
pub static mut LATCHED_INPUT: [[u8; 2]; 2] = [[0xFF, 0xFF]; 2];

#[derive(Debug, Copy, Clone, PartialEq)]
struct NextPins {
    pub set: u32,
    pub clr: u32,
}
impl NextPins {
    pub const fn new() -> Self {
        Self {
            set: 0,
            clr: 0,
        }
    }
    
    pub fn check(&mut self, input: u8, bit: usize, gpio: Gpio) {
        if ((((input >> bit) as u32) & 1) << gpio.0) > 0 {
            self.set |= 1 << gpio.0;
        } else {
            self.clr |= 1 << gpio.0;
        }
    }
}

static mut NEXT_PINS: [NextPins; 2] = [NextPins::new(); 2];
static mut STEPS: [usize; 2] = [0, 0];

const SELECT: [Gpio; 2]    = [PIN_CNT_3, PIN_CNT_1]; // CP_18 / CP_24
const UP: [Gpio; 2]        = [PIN_CNT_5, PIN_CNT_2]; // CP_8 / CP_25
const DOWN: [Gpio; 2]      = [PIN_CNT_7, PIN_CNT_4]; // CP_7 / CP_17
const LEFT_0: [Gpio; 2]    = [PIN_CNT_9, PIN_CNT_6]; // CP_6 / CP_16
const RIGHT_0: [Gpio; 2]   = [PIN_CNT_11, PIN_CNT_10]; // CP_5 / CP_15
const B_A: [Gpio; 2]       = [PIN_CNT_13, PIN_CNT_12]; // CP_4 / CP_14
const C_START: [Gpio; 2]   = [PIN_CNT_16, PIN_CNT_14]; // CP_3 / CP_13

fn initialize() {
    PIN_DETECT.set_low().into_input(false, true);
    
    for gpio in SELECT {
        gpio.set_low().into_input(false, false);
    }
    
    for gpio in [UP, DOWN, LEFT_0, RIGHT_0, B_A, C_START].as_flattened() {
        gpio.into_output(true, false).set_high();
    }
    
    unsafe {
        let inputs = INPUT_BUFFER.dequeue().unwrap_or([0xFF; 4]);
        LATCHED_INPUT = [[inputs[0], inputs[1]], [inputs[2], inputs[3]]];
        
        STEPS.fill(0);
        calc_next_edge(0);
        calc_next_edge(1);
        
        let state = calc_state(0, 0, true);
        (*SIO::ptr()).gpio_out_set().write(|w| w.bits(state.set));
        (*SIO::ptr()).gpio_out_clr().write(|w| w.bits(state.clr));
        
        displays::set_display(Port::Display0, &[LATCHED_INPUT[0][0] ^ 0xFF, LATCHED_INPUT[0][1] ^ 0xFF]);
        displays::set_display(Port::Display1, &[LATCHED_INPUT[1][0] ^ 0xFF, LATCHED_INPUT[1][1] ^ 0xFF]);
    }
}

fn enable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        for gpio in SELECT {
            gpio.clear_interrupt(Edge::EdgeHigh);
            gpio.clear_interrupt(Edge::EdgeLow);
            
            gpio.enable_interrupt(Edge::EdgeHigh);
            gpio.enable_interrupt(Edge::EdgeLow);
        }
        interrupts::enable_nvic(IO_IRQ_BANK0);
        
        
        VTABLE0.register_handler(TIMER_IRQ_0 as usize, timer_irq_0_handler);
        
        interrupts::clear_alarm(0);
        interrupts::clear_alarm(1);
        interrupts::enable_alarm(0);
        interrupts::enable_alarm(1);
        interrupts::enable_nvic(TIMER_IRQ_0);
    });
}

fn disable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        interrupts::disable_nvic(IO_IRQ_BANK0);
        interrupts::disable_nvic(TIMER_IRQ_0);
        
        for gpio in SELECT {
            gpio.disable_interrupt(Edge::EdgeHigh);
            gpio.disable_interrupt(Edge::EdgeLow);
        }
        
        interrupts::disable_alarm(0);
        interrupts::disable_alarm(1);
    });
}


pub fn run(_delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("starting Genesis replay..");
        
        enable_interrupts();
        
        while VERITAS_MODE == VeritasMode::ReplayGenesis {
            nop();
        }
        
        disable_interrupts();
        while !INPUT_BUFFER.is_empty() {
            INPUT_BUFFER.dequeue().unwrap_or_default();
        }
        REPLAY_STATE.reset();
        
        displays::set_display(Port::Display0, &[0x00, 0x00]);
        displays::set_display(Port::Display1, &[0x00, 0x00]);
        
        info!("stopped Genesis replay");
    }
}

/*#[inline(always)]
fn update_state(port: usize) {
    unsafe {
        match STEPS[port] {
            0..=4 => {
                if gpio::is_high(SELECT[port]) {
                    gpio::set_state(C_START[port],  LATCHED_INPUT[port][0] & (1 << 0) > 0);
                    gpio::set_state(B_A[port],      LATCHED_INPUT[port][0] & (1 << 1) > 0);
                    gpio::set_state(RIGHT_0[port],  LATCHED_INPUT[port][0] & (1 << 2) > 0);
                    gpio::set_state(LEFT_0[port],   LATCHED_INPUT[port][0] & (1 << 3) > 0);
                    gpio::set_state(DOWN[port],     LATCHED_INPUT[port][0] & (1 << 4) > 0);
                    gpio::set_state(UP[port],       LATCHED_INPUT[port][0] & (1 << 5) > 0);
                } else {
                    gpio::set_state(C_START[port],  LATCHED_INPUT[port][0] & (1 << 6) > 0);
                    gpio::set_state(B_A[port],      LATCHED_INPUT[port][0] & (1 << 7) > 0);
                    gpio::set_low(RIGHT_0[port]);
                    gpio::set_low(LEFT_0[port]);
                    gpio::set_state(DOWN[port],     LATCHED_INPUT[port][0] & (1 << 4) > 0);
                    gpio::set_state(UP[port],       LATCHED_INPUT[port][0] & (1 << 5) > 0);
                }
            },
            5..=6 => {
                warn!("STEPS 5-6 unimplemented!");
            },
            _ => warn!("STEPS higher than 6 reached!"),
        }
    }
}*/

#[inline(always)]
fn calc_state(port: usize, step: usize, edge: bool) -> NextPins {
    unsafe {
        let mut next = NextPins::new();
        
        match step {
            //0..=4 | 7..=8 => {
            0..=20 => {
                let latched = LATCHED_INPUT[port][0];
                if edge {
                    next.check(latched, 0, C_START[port]);
                    next.check(latched, 1, B_A[port]);
                    next.check(latched, 2, RIGHT_0[port]);
                    next.check(latched, 3, LEFT_0[port]);
                    next.check(latched, 4, DOWN[port]);
                    next.check(latched, 5, UP[port]);
                } else {
                    next.check(latched, 6, C_START[port]);
                    next.check(latched, 7, B_A[port]);
                    next.check(0, 0, RIGHT_0[port]);
                    next.check(0, 0, LEFT_0[port]);
                    next.check(latched, 4, DOWN[port]);
                    next.check(latched, 5, UP[port]);
                }
            },
            /*5..=6 => {
                let latched = LATCHED_INPUT[port][0];
                let special = LATCHED_INPUT[port][1];
                if edge {
                    next.check(latched, 0, C_START[port]);
                    next.check(latched, 1, B_A[port]);
                    next.check(special, 4, RIGHT_0[port]);
                    next.check(special, 5, LEFT_0[port]);
                    next.check(special, 6, DOWN[port]);
                    next.check(special, 7, UP[port]);
                } else {
                    next.check(latched, 6, C_START[port]);
                    next.check(latched, 7, B_A[port]);
                    next.check(0, 0, RIGHT_0[port]);
                    next.check(0, 0, LEFT_0[port]);
                    next.check(0, 0, DOWN[port]);
                    next.check(0, 0, UP[port]);
                }
            },*/
            _ => {
                warn!("STEPS higher than 20 reached!");
            },
        }
        
        next
    }
}

#[inline(always)]
pub fn calc_next_edge(port: usize) {
    unsafe {
        NEXT_PINS[port] = calc_state(port, STEPS[port] + 1, !SELECT[port].is_high());
    }
}

const SELECT_EDGE_MASK_0: u32 = (1 << (((SELECT[0].0 & 0x07) << 2) + Edge::EdgeLow as usize)) | (1 << (((SELECT[0].0 & 0x07) << 2) + Edge::EdgeHigh as usize));
//const SELECT_EDGE_MASK_1: u32 = (1 << (((SELECT[1].0 & 0x07) << 2) + Edge::EdgeLow as usize)) | (1 << (((SELECT[1].0 & 0x07) << 2) + Edge::EdgeHigh as usize));

#[unsafe(link_section = ".data")]
extern "C" fn io_irq_bank0_handler() {
    //TODO: Change logic to the following:
    //
    //  @Alarm: update_state(), and calculate next edge
    //  
    //  @GPIO: When IRQ happens, check which port, and immediately apply pre-calculated action
    //         Then calculate the next edge, increment STEPS[port], update alarm, and clear intr.
    //
    //  Use multiple bits to check for both edges at the same time.
    //  Calculated action should be applied for the entire GPIO range, not individual bits at a time!
    
    //gpio::set_high(gpio::PIN_DISPLAY_STROBE3); // DEBUG
    unsafe {
        if (*IO_BANK0::ptr()).proc0_ints(SELECT[0].0 >> 3).read().bits() & SELECT_EDGE_MASK_0 > 0 {
            (*SIO::ptr()).gpio_out_set().write(|w| w.bits(NEXT_PINS[0].set));
            (*SIO::ptr()).gpio_out_clr().write(|w| w.bits(NEXT_PINS[0].clr));
            
            STEPS[0] += 1;
            
            calc_next_edge(0);
            
            //info!("{:08X}", (*IO_BANK0::ptr()).proc0_ints[SELECT[0] >> 3].read().bits());
            interrupts::arm_alarm(0, 1500);
            SELECT[0].clear_interrupt(Edge::EdgeLow);
            SELECT[0].clear_interrupt(Edge::EdgeHigh);
        } else { // if not the first port, then must be second, no other GPIO interrupts are used
            (*SIO::ptr()).gpio_out_set().write(|w| w.bits(NEXT_PINS[1].set));
            (*SIO::ptr()).gpio_out_clr().write(|w| w.bits(NEXT_PINS[1].clr));
            
            STEPS[1] += 1;
            
            calc_next_edge(1);
            
            interrupts::arm_alarm(1, 1500);
            SELECT[1].clear_interrupt(Edge::EdgeLow);
            SELECT[1].clear_interrupt(Edge::EdgeHigh);
        }
    }
    //gpio::set_low(gpio::PIN_DISPLAY_STROBE3); // DEBUG
    
    
    /*for i in 0..=1 {
        let pin = SELECT[i];
        
        let edge = if interrupts::status_gpio_intr(pin, Edge::EdgeHigh) {
            Some(Edge::EdgeHigh)
        } else if interrupts::status_gpio_intr(pin, Edge::EdgeLow) {
            Some(Edge::EdgeLow)
        } else {
            None
        };
        
        if let Some(edge) = edge {
            unsafe { STEPS[i] += 1; }
            //delay(80);
            update_state(i);
            
            interrupts::arm_alarm(i, 1500);
            interrupts::clear_gpio_intr(pin, edge);
        }
    }*/
}

#[unsafe(link_section = ".data")]
extern "C" fn timer_irq_0_handler() {
    for port in 0..=1 {
        if interrupts::alarm_status(port) {
            unsafe {
                STEPS[port] = 0;
                
                if port == 0 {
                    let inputs = INPUT_BUFFER.dequeue().unwrap_or([0xFF; 4]);
                    LATCHED_INPUT = [[inputs[0], inputs[1]], [inputs[2], inputs[3]]];
                    
                    displays::set_display(Port::Display0, &[swap_bits(LATCHED_INPUT[0][0] ^ 0xFF, 5, 4), LATCHED_INPUT[0][1] ^ 0xFF]);
                    displays::set_display(Port::Display1, &[swap_bits(LATCHED_INPUT[1][0] ^ 0xFF, 5, 4), LATCHED_INPUT[1][1] ^ 0xFF]);
                    
                    //info!("{:02X}", LATCHED_INPUT[0][0]);
                }
                
                //let state = calc_state(0, 0, false);
                //info!("{}", format!("{state:08X?}").as_str());
                let state = calc_state(0, 0, true);
                (*SIO::ptr()).gpio_out_set().write(|w| w.bits(state.set));
                (*SIO::ptr()).gpio_out_clr().write(|w| w.bits(state.clr));
                
                calc_next_edge(port);
            }
            
            interrupts::clear_alarm(port);
        }
    }
}

#[inline(always)]
fn swap_bits(data: u8, a: usize, b: usize) -> u8 {
    let x = ((data >> a) ^ (data >> b)) & 1;
    
    data ^ ((x << a) | (x << b))
}
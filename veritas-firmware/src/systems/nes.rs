use cortex_m::asm::{delay, nop};
use cortex_m::delay::Delay;
use defmt::info;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rp2040_pac::Interrupt::{IO_IRQ_BANK0, TIMER_IRQ_0};
use rp2040_pac::TIMER;
use crate::hal::interrupts;
use crate::hal::gpio::{Gpio, PIN_CNT_11, PIN_CNT_13, PIN_CNT_18, PIN_CNT_18_DIR, PIN_CNT_5, PIN_CNT_7, PIN_CNT_9, PIN_DETECT};
use crate::hal::interrupts::Edge::{EdgeHigh, EdgeLow};
use crate::replaycore::{Transition, VERITAS_MODE, REPLAY_STATE, VeritasMode};
use crate::utilcore::displays;
use crate::utilcore::displays::Port;
use crate::{now, VTABLE0};

/// Buffered list of controller inputs. 
//pub static mut INPUT_BUFFER: Queue<[u8; 2], 1024> = Queue::new();
//pub static mut INPUT_BUFFER: Queue<[u8; 2], { 1024 * 8 }> = Queue::new();
/*pub static mut INPUTF: Lazy<(Producer<[u8; 2]>, Consumer<[u8; 2]>)> = Lazy::new(|| {
    RingBuffer::new(1024 * 8)
});*/
pub static mut INPUT_BUFFER: ConstGenericRingBuffer<[u8; 2], { 1024 * 8 }> = ConstGenericRingBuffer::new();

pub static mut LATCH_FILTER_US: u32 = 8000; //TODO: Write a detection procedure to relay to the user what the time between latch and 8th clock is.
pub static mut CLOCK_FILTER_CYC: u32 = 150;
static mut OVERREAD: u8 = 0;

static mut REPORTING: bool = false;
static mut REPORT_CYC: u32 = 0;
static mut REPORT_TIME: u64 = 0;
static mut LAST_LATCH: u64 = 0;
static mut ALARM_ACTIVATED: bool = false;
static mut FRAME_INPUT: [u8; 2] = [0xFF, 0xFF];
static mut WORKING_INPUT: [u8; 2] = [0xFF, 0xFF];

const SER: [Gpio; 2] = [PIN_CNT_5, PIN_CNT_9];
const CLK: [Gpio; 2] = [PIN_CNT_7, PIN_CNT_11];
const LAT: Gpio = PIN_CNT_13;
const RST: Gpio = PIN_CNT_18;
/// set HIGH to enable
const RST_EN: Gpio = PIN_CNT_18_DIR;

/// Prepares the device to replay a TAS.
pub fn initialize() {
    PIN_DETECT.set_low().into_input(false, true);
    
    for gpio in SER { // Player 1 and 2 serial
        gpio.into_output(false, false).set_high();
    }
    
    for gpio in CLK { // Player 1 and 2 clock
        gpio.into_input(false, false);
    }
    
    LAT.into_input(false, false); // Shared latch
    
    RST.into_output(true, false).set_low(); // Console reset (active-high)
    RST_EN.into_output(false, false).set_high();
    
    unsafe {
        FRAME_INPUT = INPUT_BUFFER.dequeue().unwrap_or([0xFF, 0xFF]);
        
        displays::set_display(Port::Display0, &[FRAME_INPUT[0] ^ 0xFF]);
        displays::set_display(Port::Display1, &[FRAME_INPUT[1] ^ 0xFF]);
        
        ALARM_ACTIVATED = false;
    }
}

#[unsafe(link_section = ".data")]
fn enable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        VTABLE0.register_handler(IO_IRQ_BANK0 as usize, io_irq_bank0_handler);
        
        CLK[0].clear_interrupt(EdgeLow);
        CLK[1].clear_interrupt(EdgeLow);
        LAT.clear_interrupt(EdgeHigh);
        
        CLK[0].enable_interrupt(EdgeLow);
        CLK[1].enable_interrupt(EdgeLow);
        LAT.enable_interrupt(EdgeHigh);
        interrupts::enable_nvic(IO_IRQ_BANK0);
        
        
        VTABLE0.register_handler(TIMER_IRQ_0 as usize, timer_irq_0_handler);
        
        interrupts::clear_alarm(0);
        
        interrupts::enable_alarm(0);
        interrupts::enable_nvic(TIMER_IRQ_0);
    });
}

#[unsafe(link_section = ".data")]
fn disable_interrupts() {
    cortex_m::interrupt::free(|_| unsafe {
        interrupts::disable_nvic(IO_IRQ_BANK0);
        interrupts::disable_nvic(TIMER_IRQ_0);
        
        CLK[0].disable_interrupt(EdgeLow);
        CLK[1].disable_interrupt(EdgeLow);
        LAT.disable_interrupt(EdgeHigh);
        
        interrupts::disable_alarm(0);
    });
}


pub fn run(delay: &mut Delay) {
    unsafe {
        initialize();
        
        info!("trans: {}", REPLAY_STATE.transitions.len());
        info!("first trans: {:?}", REPLAY_STATE.transitions.first());
        let first = INPUT_BUFFER.peek().unwrap_or(&[0xAA, 0x55]);
        info!("first input: {:02X} {:02X}", first[0], first[1]);
        
        info!("starting NES replay..");
        
        if LATCH_FILTER_US == 0 {
            REPORT_CYC = 0;
            REPORT_TIME = 0;
            REPORTING = true;
            
            enable_interrupts();
            
            while REPORT_CYC < 8 { nop(); }
            disable_interrupts();
            REPORTING = false;
            
            info!("{}, {}us", REPORT_CYC, REPORT_TIME - LAST_LATCH);
            
            LATCH_FILTER_US = (REPORT_TIME - LAST_LATCH) as u32 + 4;
        }
        
        if REPLAY_STATE.use_initial_reset {
            RST.set_high();
            delay.delay_ms(50);
            RST.set_low();
            delay.delay_ms(5);
        }
        
        enable_interrupts();
        delay.delay_us(500);
        
        
        let mut last_inputs = FRAME_INPUT;
        while VERITAS_MODE == VeritasMode::ReplayNes {
            if last_inputs != FRAME_INPUT {
                displays::set_display(Port::Display0, &[FRAME_INPUT[0] ^ 0xFF]);
                displays::set_display(Port::Display1, &[FRAME_INPUT[1] ^ 0xFF]);
                last_inputs = FRAME_INPUT;
            }
            nop();
        }
        
        disable_interrupts();
        
        for gpio in SER {
            gpio.set_high();
        }
        
        while !INPUT_BUFFER.is_empty() {
            INPUT_BUFFER.dequeue().unwrap_or_default();
        }
        REPLAY_STATE.reset();
        
        displays::set_display(Port::Display0, &[0x00]);
        displays::set_display(Port::Display1, &[0x00]);
        
        RST.set_low();
        delay.delay_ms(10);
        RST_EN.set_low();
        RST.set_high();
        
        info!("stopped NES replay");
    }
}

#[unsafe(link_section = ".data")]
#[inline(always)]
unsafe fn latch() {
    unsafe {
        LAST_LATCH = now();
        
        if !ALARM_ACTIVATED {
            ALARM_ACTIVATED = true;
            (*TIMER::ptr()).alarm0().write(|w| w.bits((*TIMER::ptr()).timerawl().read().bits().wrapping_add(LATCH_FILTER_US)));
        }
        
        WORKING_INPUT = FRAME_INPUT;
        
        // set first bit's state
        for i in 0..2 {
            if WORKING_INPUT[i] & 0x80 != 0 {
                SER[i].set_high();
            } else {
                SER[i].set_low();
            }
        }
        
        if REPORTING {
            REPORT_CYC = 0;
            REPORT_TIME = LAST_LATCH;
        }
    }
}

#[unsafe(link_section = ".data")]
#[inline(always)]
unsafe fn clock(cnt: usize) {
    unsafe {
        WORKING_INPUT[cnt] <<= 1;
        WORKING_INPUT[cnt] |= OVERREAD;
        
        //while CLK[cnt].is_low() { nop(); } // faster than responding to rising edge interrupt
        
        delay(CLOCK_FILTER_CYC); // CLOCK FILTER
        
        if WORKING_INPUT[cnt] & 0x80 != 0 {
            SER[cnt].set_high();
        } else {
            SER[cnt].set_low();
        }
        
        if REPORTING && cnt == 0 {
            REPORT_CYC += 1;
            REPORT_TIME = now();
        }
    }
}

#[unsafe(link_section = ".data")]
extern "C" fn io_irq_bank0_handler() {
    unsafe {
        if LAT.interrupt_status(EdgeHigh) {
            latch();
            
            LAT.clear_interrupt(EdgeHigh);
        } else if CLK[0].interrupt_status(EdgeLow) { // CLK[0]
            clock(0);
            
            CLK[0].clear_interrupt(EdgeLow);
        } else if CLK[1].interrupt_status(EdgeLow) { // CLK[1]
            clock(1);
            
            CLK[1].clear_interrupt(EdgeLow);
        }
    }
}

#[unsafe(link_section = ".data")]
extern "C" fn timer_irq_0_handler() {
    unsafe {
        ALARM_ACTIVATED = false;
        
        if !REPORTING {
            increment_replay();
        }
        
        interrupts::clear_alarm(0);
    }
}

#[unsafe(link_section = ".data")]
#[inline(always)]
fn increment_replay() {
    unsafe {
        if let Some(tra) = REPLAY_STATE.next_transition() {
            match tra {
                Transition::SoftReset => cortex_m::interrupt::free(|_| {
                    disable_interrupts();
                    
                    RST.set_high();
                    delay(5332558);
                    RST.set_low();
                    delay(10665);
                    
                    enable_interrupts();
                }),
                _ => (),
            }
        } else {
            FRAME_INPUT = match INPUT_BUFFER.dequeue() {
                Some(v) => v,
                None => {
                    // TODO: Attempt to add option to throw error (and stop replay) if the buffer runs out
                    
                    [0xFF, 0xFF]
                }
            };
            
            if REPLAY_STATE.index_cur == REPLAY_STATE.index_len {
                VERITAS_MODE = VeritasMode::Idle;
                info!("Replay ended!");
            } else {
                REPLAY_STATE.index_cur += 1;
            }
        }
    }
}
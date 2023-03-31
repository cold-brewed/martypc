/*

    Machine.rs
    This module defines all the parts that make up the virtual computer.
    This module also contains the main run() method that makes the CPU execute instructions and
    run devices for a given time slice.

*/
use log;

use std::{
    rc::Rc,
    cell::{Cell, RefCell}, 
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Write}
};

use crate::{
    config::{ConfigFileParams, MachineType, VideoType, ValidatorType, TraceMode},
    breakpoints::BreakPointType,
    bus::{BusInterface, MemRangeDescriptor, MEM_CP_BIT},
    cga,
    ega::{self, EGACard},
    vga::{self, VGACard},
    cpu_808x::{self, Cpu, CpuError, CpuAddress, StepResult, ServiceEvent },
    cpu_common::CpuType,
    dma::{self, DMAControllerStringState},
    fdc::{self, FloppyController},
    hdc::{self, HardDiskController},
    floppy_manager::{FloppyManager},
    vhd_manager,
    machine_manager::{MACHINE_DESCS, MachineDescriptor},
    mouse::Mouse,
    pit::{self, PitDisplayState},
    pic::{self, PicStringState},
    ppi::{self, PpiStringState},
    rom_manager::RomManager,
    sampler::{Sampler, SampleFilter},
    serial::{self, SerialPortController},
    sound::{BUFFER_MS, VOLUME_ADJUST, SoundPlayer},
    tracelogger::TraceLogger,
    videocard::{VideoCard, VideoCardState},
};

use ringbuf::{RingBuffer, Producer, Consumer};

pub const STEP_OVER_TIMEOUT: u32 = 320000;
pub const NUM_FLOPPIES: u32 = 2;
pub const NUM_HDDS: u32 = 2;

pub const MAX_MEMORY_ADDRESS: usize = 0xFFFFF;

#[derive(Copy, Clone, Debug)]
pub enum ExecutionState {
    Paused,
    BreakpointHit,
    Running,
    Halted
}

#[allow (dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum ExecutionOperation {
    None,
    Pause,
    Step,
    StepOver,
    Run,
    Reset
}

pub struct ExecutionControl {
    pub state: ExecutionState,
    op: Cell<ExecutionOperation>,
}

impl ExecutionControl {
    pub fn new() -> Self {
        Self { 
            state: ExecutionState::Paused,
            op: Cell::new(ExecutionOperation::None),
        }
    }

    pub fn set_state(&mut self, state: ExecutionState) {
        self.state = state
    }

    pub fn get_state(&self) -> ExecutionState {
        self.state
    }

    /// Sets the last execution operation.
    pub fn set_op(&mut self, op: ExecutionOperation) {

        match op {

            ExecutionOperation::Pause => {
                // Can only pause if Running
                if let ExecutionState::Running = self.state {
                    self.state = ExecutionState::Paused;
                    self.op.set(op);
                }
            }
            ExecutionOperation::Step => {
                // Can only Step if paused / breakpointhit
                if let ExecutionState::Paused | ExecutionState::BreakpointHit = self.state {
                    self.op.set(op);
                }              
            }
            ExecutionOperation::StepOver => {
                // Can only Step Over if paused / breakpointhit
                if let ExecutionState::Paused | ExecutionState::BreakpointHit = self.state {
                    self.op.set(op);
                }            
            }            
            ExecutionOperation::Run => {
                // Can only Run if paused / breakpointhit
                if let ExecutionState::Paused | ExecutionState::BreakpointHit = self.state {
                    self.op.set(op);
                } 
            }
            ExecutionOperation::Reset => {
                // Can reset anytime.
                self.op.set(op);
            }
            _ => {}
        }
        
    }

    /// Simultaneously returns the set execution operation and resets it internally to None.
    pub fn get_op(&mut self) -> ExecutionOperation {
        let op = self.op.get();
        self.op.set(ExecutionOperation::None);
        op
    }

    /// Returns the set execution operation without resetting it
    pub fn peek_op(&mut self) -> ExecutionOperation {
        self.op.get()
    }    

}

#[allow(dead_code)]
pub struct Machine<'a> 
{
    machine_type: MachineType,
    video_type: VideoType,
    audio_sampler: Sampler,
    //sound_player: SoundPlayer,
    rom_manager: RomManager,
    floppy_manager: FloppyManager,
    //bus: BusInterface,
    cpu: Cpu<'a>,
    //dma_controller: Rc<RefCell<dma::DMAController>>,
    //pit: Rc<RefCell<pit::Pit>>, 
    //speaker_buf_producer: Producer<u8>,
    //pit_buffer_consumer: Consumer<u8>,
    //pit_samples_produced: u64,
    //pit_ticks_per_sample: f64,
    //pit_ticks: f64,
    pit_log_file: Option<Box<BufWriter<File>>>,
    pit_logging_triggered: bool,
    debug_snd_file: Option<File>,
    //pic: Rc<RefCell<pic::Pic>>,
    //ppi: Rc<RefCell<ppi::Ppi>>,
    //video: Rc<RefCell<dyn VideoCard>>,
    //fdc: Rc<RefCell<FloppyController>>,
    //hdc: Rc<RefCell<HardDiskController>>,
    //serial_controller: Rc<RefCell<serial::SerialPortController>>,
    //mouse: Mouse,
    kb_buf: VecDeque<u8>,
    error: bool,
    error_str: Option<String>,
    cpu_cycles: u64,
}

impl<'a> Machine<'a> {
    pub fn new(
        config: &ConfigFileParams,
        machine_type: MachineType,
        machine_desc: &MachineDescriptor,
        trace_mode: TraceMode,
        video_type: VideoType,
        sound_player: SoundPlayer,
        rom_manager: RomManager,
        floppy_manager: FloppyManager,
        ) -> Machine<'a> 
    {


        //let mut io_bus = IoBusInterface::new();
        
        //let mut trace_file_option: Box<dyn Write + 'a> = Box::new(std::io::stdout());

        let mut trace_file_option = None;
        if config.emulator.trace_mode != TraceMode::None {
            // Open the trace file if specified
            if let Some(filename) = &config.emulator.trace_file {
                match File::create(filename) {
                    Ok(file) => {
                        trace_file_option = Some(Box::new(BufWriter::new(file)));
                    },
                    Err(e) => {
                        eprintln!("Couldn't create specified tracelog file: {}", e);
                    }
                }
            }
        }

        // Create PIT output log file if specified
        let mut pit_output_file_option = None;
        if let Some(filename) = &config.emulator.pit_output_file {
            match File::create(filename) {
                Ok(file) => {
                    pit_output_file_option = Some(Box::new(BufWriter::new(file)));
                },
                Err(e) => {
                    eprintln!("Couldn't create specified PIT log file: {}", e);
                }
            }
        }

        let mut cpu = Cpu::new(
            CpuType::Intel8088,
            trace_mode,
            trace_file_option,
            #[cfg(feature = "cpu_validator")]
            config.validator.vtype.unwrap()
        );

        let reset_vector = cpu.get_reset_vector();
        cpu.reset(reset_vector);        

        // Create sampler for PC speaker
        let sample_rate = sound_player.sample_rate() as f64;
        let sampler = Sampler::new(sample_rate, sound_player, SampleFilter::None);

        // open a file to write the sound to
        //let mut debug_snd_file = File::create("output.pcm").expect("Couldn't open debug pcm file");
        //log::trace!("Sample rate: {} pit_ticks_per_sample: {}", sample_rate, pit_ticks_per_sample);

        // Create the video trace file, if specified
        let mut video_trace = TraceLogger::None;
        if let Some(trace_filename) = &config.emulator.video_trace_file {
            video_trace = TraceLogger::from_filename(&trace_filename);
        }

        // Install devices
        cpu.bus_mut().install_devices(video_type, machine_desc, video_trace);

        // Load BIOS ROM images
        rom_manager.copy_into_memory(cpu.bus_mut());

        // Load checkpoint flags into memory
        rom_manager.install_checkpoints(cpu.bus_mut());

        // Set entry point for ROM (mostly used for diagnostic ROMs that don't have a FAR JUMP reset vector)
    
        let rom_entry_point = rom_manager.get_entrypoint();
        cpu.set_reset_vector(CpuAddress::Segmented(rom_entry_point.0, rom_entry_point.1));
        cpu.reset_address();

        Machine {
            machine_type,
            video_type,
            audio_sampler: sampler,
            //sound_player,
            rom_manager,
            floppy_manager,
            //bus: bus,
            cpu,
            //dma_controller: dma,
            //pit,
            //speaker_buf_producer,
            //pit_buffer_consumer: speaker_buf_consumer,
            //pit_ticks_per_sample,
            //pit_ticks: 0.0,
            //pit_samples_produced: 0,
            pit_log_file: pit_output_file_option,
            pit_logging_triggered: false,
            debug_snd_file: None,
            //pic,
            //ppi,
            //video,
            //fdc,
            //hdc,
            //serial_controller: serial,
            //mouse,
            kb_buf: VecDeque::new(),
            error: false,
            error_str: None,
            cpu_cycles: 0,
        }
    }

    pub fn bus(&self) -> &BusInterface {
        self.cpu.bus()
    }

    pub fn bus_mut(&mut self) -> &mut BusInterface {
        self.cpu.bus_mut()
    }

    //pub fn cga(&self) -> Rc<RefCell<CGACard>> {
    //    self.cga.clone()
    //}

    pub fn videocard(&mut self) -> Option<Box<&mut dyn VideoCard>> {
        self.cpu.bus_mut().video_mut()
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn fdc(&mut self) -> &mut Option<FloppyController> {
        self.cpu.bus_mut().fdc_mut()
    }

    pub fn hdc(&mut self) -> &mut Option<HardDiskController> {
        self.cpu.bus_mut().hdc_mut()
    }

    pub fn floppy_manager(&self) -> &FloppyManager {
        &self.floppy_manager
    }

    pub fn cpu_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    /// Return the number of cycles the PIT has ticked.
    pub fn pit_cycles(&self) -> u64 {
        // Safe to unwrap pit as a PIT will always exist on any machine type
        self.cpu.bus().pit().as_ref().unwrap().get_cycles()
    }

    /// Return the PIT's state as a PitDisplaySate struct. 
    /// This is a mutable function as receiving the display state resets the various
    /// state variable's dirty flags.
    pub fn pit_state(&mut self) -> PitDisplayState {
        // Safe to unwrap pit as a PIT will always exist on any machine type
        let pit = self.cpu.bus_mut().pit_mut().as_mut().unwrap();
        let pit_data = pit.get_display_state(true);
        pit_data
    }

    /*
    pub fn get_pit_buf(&self) -> Vec<u8> {
        let (a,b) = self.pit_buffer_consumer.as_slices();

        a.iter().cloned().chain(b.iter().cloned()).collect()
    }
    */

    pub fn pic_state(&mut self) -> PicStringState {
        // There will always be a primary PIC, so safe to unwrap.
        // TODO: Handle secondary PIC if present.
        self.cpu.bus_mut().pic_mut().as_mut().unwrap().get_string_state()
    }

    pub fn ppi_state(&mut self) -> Option<PpiStringState> {

        if let Some(ppi) = self.cpu.bus_mut().ppi_mut() {
            Some(ppi.get_string_state())
        }
        else {
            None
        }
    }

    pub fn dma_state(&mut self) -> DMAControllerStringState {
        // There will always be a primary DMA, so safe to unwrap.
        // TODO: Handle secondary DMA if present.
        self.cpu.bus_mut().dma_mut().as_mut().unwrap().get_string_state()
    }
    
    pub fn videocard_state(&mut self) -> Option<VideoCardState> {
        if let Some(video_card) = self.cpu.bus_mut().video_mut() {
            // A video card is present
            Some(video_card.get_videocard_string_state())
        }
        else {
            // no video card
            None
        }
    }

    pub fn get_error_str(&self) -> &Option<String> {
        &self.error_str
    }

    pub fn key_press(&mut self, code: u8) {
        self.kb_buf.push_back(code);
    }

    pub fn key_release(&mut self, code: u8 ) {
        // HO Bit set converts a scancode into its 'release' code
        self.kb_buf.push_back(code | 0x80);
    }

    pub fn mouse_mut(&mut self) -> &mut Option<Mouse> {
        self.cpu.bus_mut().mouse_mut()
    }

    pub fn bridge_serial_port(&mut self, port_num: usize, port_name: String) {

        if let Some(spc) = self.cpu.bus_mut().serial_mut() {
            if let Err(e) = spc.bridge_port(port_num, port_name) {
                log::error!("Failed to bridge serial port: {}", e );
            }
        }
        else {
            log::error!("No serial port controller present!");
        }
    }

    pub fn set_breakpoints(&mut self, bp_list: Vec<BreakPointType>) {
        self.cpu.set_breakpoints(bp_list)
    }

    pub fn reset(&mut self) {

        // Clear any error state.
        self.error = false;
        self.error_str = None;

        // Reset CPU.
        self.cpu.reset(CpuAddress::Segmented(0xFFFF, 0x0000));

        // Clear RAM
        self.cpu.bus_mut().clear();

        // Reload BIOS ROM images
        self.rom_manager.copy_into_memory(self.cpu.bus_mut());
        // Clear patch installation status
        self.rom_manager.reset_patches();

        // Reset all installed devices.
        self.cpu.bus_mut().reset_devices();
    }

    fn cycles_to_us(&self, cycles: u32) -> f64 {

        1.0 / cpu_808x::CPU_MHZ * cycles as f64
    }
    
    pub fn run(&mut self, cycle_target: u32, exec_control: &mut ExecutionControl) -> u64 {

        let mut kb_event_processed = false;
        let mut skip_breakpoint = false;
        let mut instr_count = 0;

        // Was reset requested?
        if let ExecutionOperation::Reset = exec_control.peek_op() {
            _ = exec_control.get_op(); // Clear the reset operation
            self.reset();
            exec_control.state = ExecutionState::Paused;
            return 0
        }

        let mut step_over = false;
        let cycle_target_adj = match exec_control.state {
            ExecutionState::Paused => {
                match exec_control.get_op() {
                    ExecutionOperation::Step => {
                        // Skip current breakpoint, if any
                        skip_breakpoint = true;
                        // Execute 1 cycle
                        1
                    },
                    ExecutionOperation::StepOver => {
                        // Skip current breakpoint, if any
                        skip_breakpoint = true;
                        // Set step-over flag
                        step_over = true;
                        // Execute 1 cycle
                        1                        
                    }
                    ExecutionOperation::Run => {
                        // Transition to ExecutionState::Running
                        exec_control.state = ExecutionState::Running;
                        cycle_target
                    },                      
                    _ => return 0
                }
            
            },
            ExecutionState::Running => {
                _ = exec_control.get_op(); // Clear any pending operation
                cycle_target
            },
            ExecutionState::BreakpointHit => {
                match exec_control.get_op() {
                    ExecutionOperation::Step => {
                        log::trace!("BreakpointHit -> Step");
                        // Clear CPU's breakpoint flag
                        self.cpu.clear_breakpoint_flag();
                        // Skip current breakpoint, if any
                        skip_breakpoint = true;
                        // Transition to ExecutionState::Paused
                        exec_control.state = ExecutionState::Paused;

                        // Execute one instruction only
                        1
                    },
                    ExecutionOperation::StepOver => {
                        log::trace!("BreakpointHit -> StepOver");
                        // Clear CPU's breakpoint flag
                        self.cpu.clear_breakpoint_flag();
                        // Skip current breakpoint, if any
                        skip_breakpoint = true;
                        // Set the step over flag
                        step_over = true;
                        // Transition to ExecutionState::Paused
                        exec_control.state = ExecutionState::Paused;

                        // Execute one instruction only
                        1
                    },
                    ExecutionOperation::Run => {
                        // Clear CPU's breakpoint flag
                        self.cpu.clear_breakpoint_flag();
                        // Skip current breakpoint, if any
                        skip_breakpoint = true;
                        // Transition to ExecutionState::Running
                        exec_control.state = ExecutionState::Running;
                        cycle_target
                    },                    
                    _ => return 0
                }

            },
            ExecutionState::Halted => {
                match exec_control.get_op() {
                    ExecutionOperation::Run => {
                        // Transition to ExecutionState::Running
                        exec_control.state = ExecutionState::Running;
                        cycle_target
                    }
                    _ => return 0
                }
            }
        };

        let mut cycles_elapsed = 0;

        while cycles_elapsed < cycle_target_adj {

            let fake_cycles: u32 = 7;
            let mut cpu_cycles;

            if self.cpu.is_error() {
                break;
            }

            let flat_address = self.cpu.get_linear_ip();

            // Match checkpoints
            if self.cpu.bus().get_flags(flat_address as usize) & MEM_CP_BIT != 0 {
                if let Some(cp) = self.rom_manager.get_checkpoint(flat_address) {
                    log::trace!("ROM CHECKPOINT: [{:05X}] {}", flat_address, cp);
                }

                // Check for patching checkpoint & install patches
                if self.rom_manager.is_patch_checkpoint(flat_address) {
                    log::trace!("ROM PATCH CHECKPOINT: [{:05X}] Installing ROM patches...", flat_address);
                    self.rom_manager.install_patch(self.cpu.bus_mut(), flat_address);
                }
            }
            
            let mut step_over_target = None;

            match self.cpu.step(skip_breakpoint) {
                Ok((step_result, step_cycles)) => {

                    match step_result {
                        StepResult::Normal => {
                            cpu_cycles = step_cycles;
                        },
                        StepResult::Call(target) => {
                            cpu_cycles = step_cycles;
                            step_over_target = Some(target);
                        }
                        StepResult::BreakpointHit => {
                            exec_control.state = ExecutionState::BreakpointHit;
                            return 1
                        }
                    }
                    
                },
                Err(err) => {
                    if let CpuError::CpuHaltedError(_) = err {
                        log::error!("CPU Halted!");
                        exec_control.state = ExecutionState::Halted;
                    }
                    self.error = true;
                    self.error_str = Some(format!("{}", err));
                    log::error!("CPU Error: {}\n{}", err, self.cpu.dump_instruction_history_string());
                    cpu_cycles = 0
                } 
            }

            instr_count += 1;
            cycles_elapsed += cpu_cycles;
            self.cpu_cycles += cpu_cycles as u64;            

            if cpu_cycles == 0 {
                log::warn!("Instruction returned 0 cycles");
                cpu_cycles = fake_cycles;
            }

            self.run_devices(cpu_cycles, &mut kb_event_processed);

            // If we returned a step over target address, execution is paused, and step over was requested, 
            // then consume as many instructions as needed to get to to the 'next' instruction. This will
            // skip over any CALL or interrupt encountered.
            if step_over {
                if let Some(step_over_target) = step_over_target {

                    log::debug!("Step over requested for CALL, return addr: {}", step_over_target );
                    let mut cs_ip = self.cpu.get_csip();
                    let mut step_over_cycles = 0;

                    while cs_ip != step_over_target {

                        match self.cpu.step(skip_breakpoint) {
                            Ok((step_result, step_cycles)) => {
            
                                match step_result {
                                    StepResult::Normal => {
                                        cpu_cycles = step_cycles
                                    },
                                    StepResult::Call(_) => {
                                        cpu_cycles = step_cycles
                                        // We are already stepping over a base CALL instruction, so ignore futher CALLS/interrupts.
                                    }
                                    StepResult::BreakpointHit => {
                                        // We can hit an 'inner' breakpoint while stepping over. This is fine, and ends the step
                                        // over operation at the breakpoint.
                                        exec_control.state = ExecutionState::BreakpointHit;
                                        return instr_count
                                    }
                                }
                            },
                            Err(err) => {
                                if let CpuError::CpuHaltedError(_) = err {
                                    log::error!("CPU Halted!");
                                    exec_control.state = ExecutionState::Halted;
                                }
                                self.error = true;
                                self.error_str = Some(format!("{}", err));
                                log::error!("CPU Error: {}\n{}", err, self.cpu.dump_instruction_history_string());
                                cpu_cycles = 0
                            } 
                        }

                        instr_count += 1;
                        cycles_elapsed += cpu_cycles;
                        self.cpu_cycles += cpu_cycles as u64;            

                        step_over_cycles += cpu_cycles;
            
                        if cpu_cycles == 0 {
                            log::warn!("Instruction returned 0 cycles");
                            cpu_cycles = fake_cycles;
                        }
            
                        self.run_devices(cpu_cycles, &mut kb_event_processed);

                        cs_ip = self.cpu.get_csip();

                        if step_over_cycles > STEP_OVER_TIMEOUT {
                            log::warn!("Step over operation timed out: No return after {} cycles.", STEP_OVER_TIMEOUT);
                            break;
                        }
                    }
                }
            }

            if let Some(event) = self.cpu.get_service_event() {
                match event {
                    ServiceEvent::TriggerPITLogging => {
                        log::debug!("PIT logging has been triggered.");
                        self.pit_logging_triggered = true;
                    }
                }
            }
        }

        instr_count
    }

    pub fn run_devices(&mut self, cpu_cycles: u32, kb_event_processed: &mut bool) {

        // Convert cycles into elapsed microseconds
        let us;
        us = self.cycles_to_us(cpu_cycles);

        // Process a keyboard event once per frame.
        // A reasonably fast typist can generate two events in a single 16ms frame, and to the virtual cpu
        // they then appear to happen instantenously. The PPI has no buffer, so one scancode gets lost. 
        // 
        // If we limit keyboard events to once per frame, this avoids this problem. I'm a reasonably
        // fast typist and this method seems to work fine.
        let mut kb_byte_opt: Option<u8> = None;
        if self.kb_buf.len() > 0 && !*kb_event_processed {

            kb_byte_opt = self.kb_buf.pop_front();
            if kb_byte_opt.is_some() {
                *kb_event_processed = true;
            }
        }

        // Tick the sampler.
        self.audio_sampler.tick(us);

        // Instruct Bus to run installed devices.
        // We send the IO bus the elapsed time in us, and a mutable reference to the audio sampler so that the 
        // PIT can produce sound via the PC speaker.
        self.cpu.bus_mut().run_devices(us, kb_byte_opt, &mut self.audio_sampler);

        // Sample the PIT channel
        /*
        self.pit_ticks += cpu_cycles as f64;
        while self.pit_ticks >= self.pit_ticks_per_sample {
            self.pit_buf_to_sound_buf();
            self.pit_ticks -= self.pit_ticks_per_sample;
        }
        */
    }

    /// Called to update machine once per frame.
    /// Mostly used for serial passthrouh function to synchronize virtual
    /// serial port with real serial port.
    pub fn frame_update(&mut self) {

        // Update serial port, if present
        if let Some(spc) =  self.cpu.bus_mut().serial_mut() {
            spc.update();
        }  
    }

    pub fn play_sound_buffer(&self) {
        self.audio_sampler.play();
    }

    /*
    pub fn pit_buf_to_sound_buf(&mut self) {

        let pit_ticks: usize = self.pit_ticks_per_sample as usize;
        if self.pit_buffer_consumer.len() < pit_ticks {
            return
        }

        let mut sum = 0;
        let mut sample;
        let mut samples_read = false;

        // If logging enabled, read samples and log to file.
        if let Some(file) = self.pit_log_file.as_mut() {
            if self.pit_logging_triggered {
                for _ in 0..pit_ticks {

                    sample = match self.pit_buffer_consumer.pop() {
                        Some(s) => s,
                        None => {
                            log::trace!("No byte in pit buffer");
                            0
                        }
                    };
                    sum += sample;

                    let sample_f32: f32 = if sample == 0 { 0.0 } else { 1.0 };
                    file.write(&sample_f32.to_le_bytes()).expect("Error writing to debug sound file");

                }
                samples_read = true;
            }
        }

        // Otherwise, just read samples
        if !samples_read {
            for _ in 0..pit_ticks {
            
                sample = match self.pit_buffer_consumer.pop() {
                    Some(s) => s,
                    None => {
                        log::trace!("No byte in pit buffer");
                        0
                    }
                };
                sum += sample;
            }
        }

        let average: f32 = sum as f32 / pit_ticks as f32;

        //log::trace!("Sample: sum: {}, ticks: {}, avg: {}", sum, pit_ticks, average);

        self.pit_samples_produced += 1;
        //log::trace!("producer: {}", self.pit_samples_produced);

        self.sound_player.queue_sample(average as f32 * VOLUME_ADJUST);
    
    }
    */


}
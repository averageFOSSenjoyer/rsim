use crate::backend::component::alu::Alu;
use crate::backend::component::alu::AluMux1;
use crate::backend::component::alu::AluMux2;
use crate::backend::component::cmp::Cmp;
use crate::backend::component::cmp::CmpMux;
use crate::backend::component::control::Control;
use crate::backend::component::data_out::DataOut;
use crate::backend::component::ir::IR;
use crate::backend::component::mar::Mar;
use crate::backend::component::mar::MarMux;
use crate::backend::component::mdr::Mdr;
use crate::backend::component::mem_ctl::MemCtl;
use crate::backend::component::pc::Pc;
use crate::backend::component::pc::PcMux;
use crate::backend::component::regfile::RegFile;
use crate::backend::component::regfile::RegFileMux;
use crate::backend::core::LinkType::*;
use crate::backend::core::StatsType::InstructionsRan;
use crate::backend::util::byte::Bytes;
use crate::backend::util::types::Byte;
use crate::backend::util::types::States;
use crate::backend::util::types::Word;
use crossbeam_channel::{unbounded, Receiver, Sender};
use rsim_core::component::Component;
use rsim_core::event::Event;
use rsim_core::sim_dispatcher::SimDispatcher;
use rsim_core::sim_manager::SimManager;
use rsim_core::types::EventId;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use strum::EnumIter;
use strum::IntoEnumIterator;

type LinksMap = HashMap<LinkType, (Sender<Box<dyn Event>>, Receiver<Box<dyn Event>>)>;

#[derive(EnumIter, Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum StatsType {
    InstructionsRan,
}

#[derive(EnumIter, Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
/// The naming convention for LinkType is as follows:
/// <source component>_<destination component>_<destination port name>
enum LinkType {
    Control_Alu_AluOp,
    Control_AluMux1_Sel,
    Control_AluMux2_Sel,
    Control_Cmp_Op,
    Control_Mar_Load,
    Control_Mdr_Load,
    Control_Pc_Load,
    Control_Ir_Load,
    Control_Regfile_Load,
    Control_Dataout_Load,
    Control_PcMux_Sel,
    Control_RegfileMux_Sel,
    Control_MarMux_Sel,
    Control_CmpMux_Sel,
    Control_MemCtl_MemRead,
    Control_MemCtl_MemWrite,
    Control_MemCtl_MemRmask,
    Control_MemCtl_MemWmask,
    Ir_Control_Funct3,
    Ir_Control_Funct7,
    Ir_Control_Opcode,
    Ir_Regfile_Rs1Idx,
    Ir_Regfile_Rs2Idx,
    Ir_Regfile_RdIdx,
    Ir_AluMux2_BImm,
    Ir_AluMux2_SImm,
    Ir_AluMux2_JImm,
    Ir_AluMux2_IImm,
    Ir_CmpMux_IImm,
    Ir_AluMux2_UImm,
    Ir_RegfileMux_UImm,
    CmpMux_Cmp_B,
    Cmp_Control_Out,
    Cmp_RegfileMux_Out,
    RegfileMux_Regfile_RdData,
    Regfile_AluMux1_Rs1Data,
    Regfile_Cmp_Rs1Data,
    Regfile_AluMux2_Rs2Data,
    Regfile_CmpMux_Rs2Data,
    Regfile_DataOut_Rs2Data,
    MarMux_Mar_Data,
    Mar_Control_Out,
    Mar_RegfileMux_Out,
    Mar_DataOut_Out,
    Mar_MemCtl_Out,
    Mdr_Ir_Out,
    Mdr_RegfileMux_Out,
    PcMux_Pc_Data,
    Pc_AluMux1_Out,
    Pc_PcMux_Out,
    Pc_MarMux_Out,
    Pc_RegfileMux_Out,
    AluMux1_Alu_A,
    AluMux2_Alu_B,
    Alu_PcMux_Out,
    Alu_MarMux_Out,
    Alu_RegfileMux_Out,
    MemCtl_Mdr_Rdata,
    MemCtl_Control_Resp,
    DataOut_MemCtl_Data,
}

/// A wrapper for all the components
#[allow(dead_code)]
pub struct Core {
    ack_channel: (Sender<EventId>, Receiver<EventId>),
    pub(crate) sim_manager: Arc<SimManager>,
    sim_dispatcher_handlers: Vec<JoinHandle<()>>,
    mem_ctl: Arc<Mutex<MemCtl>>,
    control: Arc<Mutex<Control>>,
    ir: Arc<Mutex<IR>>,
    pc_mux: Arc<Mutex<PcMux>>,
    pc: Arc<Mutex<Pc>>,
    mar_mux: Arc<Mutex<MarMux>>,
    mar: Arc<Mutex<Mar>>,
    mdr: Arc<Mutex<Mdr>>,
    alu_mux1: Arc<Mutex<AluMux1>>,
    alu_mux2: Arc<Mutex<AluMux2>>,
    alu: Arc<Mutex<Alu>>,
    cmp_mux: Arc<Mutex<CmpMux>>,
    cmp: Arc<Mutex<Cmp>>,
    regfile_mux: Arc<Mutex<RegFileMux>>,
    regfile: Arc<Mutex<RegFile>>,
    data_out: Arc<Mutex<DataOut>>,
    links: LinksMap,
    commit_file: Option<File>,
    stats: HashMap<StatsType, u128>,
}

impl Core {
    fn log_commits(&mut self) {
        if let Some(commit_file) = self.commit_file.as_mut() {
            // locking is fine here, we are not advancing the sim
            let control = self.control.lock().unwrap();
            let pc = self.pc.lock().unwrap();
            let ir = self.ir.lock().unwrap();
            let regfile = self.regfile.lock().unwrap();
            let mar = self.mar.lock().unwrap();
            let mem_ctl = self.mem_ctl.lock().unwrap();

            if control.state == States::Fetch1
                || control.state == States::Fetch2
                || control.state == States::Fetch3
                || control.state == States::Decode
                || control.state == States::Store1
                || control.state == States::Load1
                || control.state == States::AddrCalc
            {
                return;
            }

            let mut line = String::new();

            line.push_str(&format!(
                "core   0: 3 0x{} (0x{})",
                pc.data_inner, ir.data_inner
            ));

            if regfile.rd_wr.is_something_nonzero() && ir.get_rd_idx().is_something_nonzero() {
                let raw_rd: u8 = Into::<Option<u8>>::into(ir.get_rd_idx()).unwrap();
                if raw_rd < 10 {
                    line.push_str(&format!(" x{}  ", raw_rd))
                } else {
                    line.push_str(&format!(" x{} ", raw_rd))
                }
                line.push_str(&format!("0x{}", regfile.rd_data));
            }

            if control.state == States::Load2 && control.get_rmask().is_something_nonzero() {
                let rmask = Into::<Option<u8>>::into(control.get_rmask()).unwrap();
                let mut byte_shift = 0;
                for i in 0..4u8 {
                    if (rmask >> i) & 0x1 == 0x1 {
                        byte_shift = i;
                        break;
                    }
                }
                line.push_str(&format!(
                    " mem 0x{}",
                    (mar.data_inner & Word::from(0xFFFFFFFCu32)) + Byte::from(byte_shift)
                ));
            }

            if control.state == States::Store2 && control.get_wmask().is_something_nonzero() {
                let wmask = Into::<Option<u8>>::into(control.get_wmask()).unwrap();
                let mut byte_shift = 0;
                for i in 0..4u8 {
                    if (wmask >> i) & 0x1 == 0x1 {
                        byte_shift = i;
                        break;
                    }
                }
                let mut byte_count = 0;
                for i in 0..4u8 {
                    if (wmask >> i) & 0x1 == 0x1 {
                        byte_count += 1;
                    }
                }

                line.push_str(&format!(
                    " mem 0x{}",
                    (mar.data_inner & Word::from(0xFFFFFFFCu32)) + Byte::from(byte_shift)
                ));
                if let Some(data_out) = Into::<Option<u32>>::into(mem_ctl.cpu_wdata) {
                    let shifted_data = data_out >> (8 * byte_shift);
                    let data_out_str = match byte_count {
                        1 => {
                            format!("{}", Byte::from(shifted_data as u8))
                        }
                        2 => {
                            format!("{}", Bytes::<2>::from(shifted_data as u16))
                        }
                        4 => {
                            format!("{}", Word::from(shifted_data))
                        }
                        _ => "".to_string(),
                    };
                    line.push_str(&format!(" 0x{}", data_out_str));
                }
            }

            line.push('\n');
            commit_file.write_all(line.as_bytes()).unwrap();

            let instructions_ran = self.stats[&InstructionsRan];
            if instructions_ran % 1000 == 0 {
                println!("commit #{}", instructions_ran);
                print!("{}", line);
            }
        }
    }

    pub fn run_cycle(&mut self) {
        self.sim_manager.run_cycle().unwrap();
        self.sim_manager.run_cycle_end().unwrap();
        self.log_commits();
    }

    pub fn run_instruction(&mut self) {
        let old_pc = self.pc.lock().unwrap().data_inner;

        while !self.ir.lock().unwrap().can_end() && old_pc == self.pc.lock().unwrap().data_inner {
            self.run_cycle()
        }

        self.stats
            .insert(InstructionsRan, self.stats[&InstructionsRan] + 1);
    }

    pub fn run_end(&mut self) {
        while !self.ir.lock().unwrap().can_end() {
            self.run_instruction()
        }
    }

    pub fn load_bin(&mut self, data: &[u8], addr: Word) {
        self.mem_ctl.lock().unwrap().load_bin(data, addr);
    }

    pub fn new(threads_to_use: usize, commit_file: Option<File>) -> Self {
        let ack_channel = unbounded();
        let sim_manager = SimManager::new(ack_channel.1.clone());
        let mut links: LinksMap = Default::default();
        let mut stats: HashMap<StatsType, u128> = Default::default();

        for link_type in LinkType::iter() {
            links.insert(link_type, unbounded());
        }
        for stats_type in StatsType::iter() {
            stats.insert(stats_type, 0u128);
        }

        let mem_ctl = Arc::new(Mutex::new(MemCtl::new(
            0,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Mar_MemCtl_Out].1.clone(),
            links[&DataOut_MemCtl_Data].1.clone(),
            links[&Control_MemCtl_MemRead].1.clone(),
            links[&Control_MemCtl_MemRmask].1.clone(),
            links[&Control_MemCtl_MemWrite].1.clone(),
            links[&Control_MemCtl_MemWmask].1.clone(),
            links[&MemCtl_Mdr_Rdata].0.clone(),
            links[&MemCtl_Control_Resp].0.clone(),
        )));

        let control = Arc::new(Mutex::new(Control::new(
            1,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Ir_Control_Funct3].1.clone(),
            links[&Ir_Control_Funct7].1.clone(),
            links[&Cmp_Control_Out].1.clone(),
            links[&Ir_Control_Opcode].1.clone(),
            links[&Mar_Control_Out].1.clone(),
            links[&MemCtl_Control_Resp].1.clone(),
            links[&Control_Mar_Load].0.clone(),
            links[&Control_Mdr_Load].0.clone(),
            links[&Control_Pc_Load].0.clone(),
            links[&Control_Ir_Load].0.clone(),
            links[&Control_Regfile_Load].0.clone(),
            links[&Control_Dataout_Load].0.clone(),
            links[&Control_Alu_AluOp].0.clone(),
            links[&Control_Cmp_Op].0.clone(),
            links[&Control_PcMux_Sel].0.clone(),
            links[&Control_AluMux1_Sel].0.clone(),
            links[&Control_AluMux2_Sel].0.clone(),
            links[&Control_RegfileMux_Sel].0.clone(),
            links[&Control_MarMux_Sel].0.clone(),
            links[&Control_CmpMux_Sel].0.clone(),
            links[&Control_MemCtl_MemRead].0.clone(),
            links[&Control_MemCtl_MemWrite].0.clone(),
            links[&Control_MemCtl_MemWmask].0.clone(),
            links[&Control_MemCtl_MemRmask].0.clone(),
        )));

        let ir = Arc::new(Mutex::new(IR::new(
            2,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Control_Ir_Load].1.clone(),
            links[&Mdr_Ir_Out].1.clone(),
            links[&Ir_Control_Funct3].0.clone(),
            links[&Ir_Control_Funct7].0.clone(),
            links[&Ir_Control_Opcode].0.clone(),
            links[&Ir_AluMux2_IImm].0.clone(),
            links[&Ir_CmpMux_IImm].0.clone(),
            links[&Ir_AluMux2_SImm].0.clone(),
            links[&Ir_AluMux2_BImm].0.clone(),
            links[&Ir_AluMux2_UImm].0.clone(),
            links[&Ir_RegfileMux_UImm].0.clone(),
            links[&Ir_AluMux2_JImm].0.clone(),
            links[&Ir_Regfile_Rs1Idx].0.clone(),
            links[&Ir_Regfile_Rs2Idx].0.clone(),
            links[&Ir_Regfile_RdIdx].0.clone(),
        )));

        let pc_mux = Arc::new(Mutex::new(PcMux::new(
            3,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Pc_PcMux_Out].1.clone(),
            links[&Alu_PcMux_Out].1.clone(),
            links[&Control_PcMux_Sel].1.clone(),
            links[&PcMux_Pc_Data].0.clone(),
        )));

        let pc = Arc::new(Mutex::new(Pc::new(
            4,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Control_Pc_Load].1.clone(),
            links[&PcMux_Pc_Data].1.clone(),
            links[&Pc_AluMux1_Out].0.clone(),
            links[&Pc_PcMux_Out].0.clone(),
            links[&Pc_MarMux_Out].0.clone(),
            links[&Pc_RegfileMux_Out].0.clone(),
        )));

        let mar_mux = Arc::new(Mutex::new(MarMux::new(
            5,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Pc_MarMux_Out].1.clone(),
            links[&Alu_MarMux_Out].1.clone(),
            links[&Control_MarMux_Sel].1.clone(),
            links[&MarMux_Mar_Data].0.clone(),
        )));

        let mar = Arc::new(Mutex::new(Mar::new(
            6,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Control_Mar_Load].1.clone(),
            links[&MarMux_Mar_Data].1.clone(),
            links[&Mar_Control_Out].0.clone(),
            links[&Mar_RegfileMux_Out].0.clone(),
            links[&Mar_DataOut_Out].0.clone(),
            links[&Mar_MemCtl_Out].0.clone(),
        )));

        let mdr = Arc::new(Mutex::new(Mdr::new(
            7,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Control_Mdr_Load].1.clone(),
            links[&MemCtl_Mdr_Rdata].1.clone(),
            links[&Mdr_Ir_Out].0.clone(),
            links[&Mdr_RegfileMux_Out].0.clone(),
        )));

        let alu_mux1 = Arc::new(Mutex::new(AluMux1::new(
            8,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Regfile_AluMux1_Rs1Data].1.clone(),
            links[&Pc_AluMux1_Out].1.clone(),
            links[&Control_AluMux1_Sel].1.clone(),
            links[&AluMux1_Alu_A].0.clone(),
        )));

        let alu_mux2 = Arc::new(Mutex::new(AluMux2::new(
            9,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Ir_AluMux2_IImm].1.clone(),
            links[&Ir_AluMux2_UImm].1.clone(),
            links[&Ir_AluMux2_BImm].1.clone(),
            links[&Ir_AluMux2_SImm].1.clone(),
            links[&Ir_AluMux2_JImm].1.clone(),
            links[&Regfile_AluMux2_Rs2Data].1.clone(),
            links[&Control_AluMux2_Sel].1.clone(),
            links[&AluMux2_Alu_B].0.clone(),
        )));

        let alu = Arc::new(Mutex::new(Alu::new(
            10,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&AluMux1_Alu_A].1.clone(),
            links[&AluMux2_Alu_B].1.clone(),
            links[&Control_Alu_AluOp].1.clone(),
            links[&Alu_PcMux_Out].0.clone(),
            links[&Alu_MarMux_Out].0.clone(),
            links[&Alu_RegfileMux_Out].0.clone(),
        )));

        let cmp_mux = Arc::new(Mutex::new(CmpMux::new(
            11,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Regfile_CmpMux_Rs2Data].1.clone(),
            links[&Ir_CmpMux_IImm].1.clone(),
            links[&Control_CmpMux_Sel].1.clone(),
            links[&CmpMux_Cmp_B].0.clone(),
        )));

        let cmp = Arc::new(Mutex::new(Cmp::new(
            12,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Regfile_Cmp_Rs1Data].1.clone(),
            links[&CmpMux_Cmp_B].1.clone(),
            links[&Control_Cmp_Op].1.clone(),
            links[&Cmp_Control_Out].0.clone(),
            links[&Cmp_RegfileMux_Out].0.clone(),
        )));

        let regfile_mux = Arc::new(Mutex::new(RegFileMux::new(
            13,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Alu_RegfileMux_Out].1.clone(),
            links[&Cmp_RegfileMux_Out].1.clone(),
            links[&Ir_RegfileMux_UImm].1.clone(),
            links[&Mar_RegfileMux_Out].1.clone(),
            links[&Mdr_RegfileMux_Out].1.clone(),
            links[&Pc_RegfileMux_Out].1.clone(),
            links[&Control_RegfileMux_Sel].1.clone(),
            links[&RegfileMux_Regfile_RdData].0.clone(),
        )));

        let regfile = Arc::new(Mutex::new(RegFile::new(
            14,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Ir_Regfile_Rs1Idx].1.clone(),
            links[&Ir_Regfile_Rs2Idx].1.clone(),
            links[&Control_Regfile_Load].1.clone(),
            links[&Ir_Regfile_RdIdx].1.clone(),
            links[&RegfileMux_Regfile_RdData].1.clone(),
            links[&Regfile_AluMux1_Rs1Data].0.clone(),
            links[&Regfile_Cmp_Rs1Data].0.clone(),
            links[&Regfile_AluMux2_Rs2Data].0.clone(),
            links[&Regfile_CmpMux_Rs2Data].0.clone(),
            links[&Regfile_DataOut_Rs2Data].0.clone(),
        )));

        let data_out = Arc::new(Mutex::new(DataOut::new(
            15,
            sim_manager.clone(),
            ack_channel.0.clone(),
            links[&Control_Dataout_Load].1.clone(),
            links[&Mar_DataOut_Out].1.clone(),
            links[&Regfile_DataOut_Rs2Data].1.clone(),
            links[&DataOut_MemCtl_Data].0.clone(),
        )));

        let components_vec: Vec<Arc<Mutex<dyn Component>>> = vec![
            mem_ctl.clone(),
            control.clone(),
            ir.clone(),
            pc_mux.clone(),
            pc.clone(),
            mar_mux.clone(),
            mar.clone(),
            mdr.clone(),
            alu_mux1.clone(),
            alu_mux2.clone(),
            alu.clone(),
            cmp_mux.clone(),
            cmp.clone(),
            regfile_mux.clone(),
            regfile.clone(),
            data_out.clone(),
        ];

        let sim_dispatchers: Vec<_> = components_vec
            .chunks((components_vec.len() as f32 / threads_to_use as f32).ceil() as usize)
            .map(|component| SimDispatcher::new(Arc::downgrade(&sim_manager), component.into()))
            .collect();
        sim_dispatchers.iter().for_each(|s| s.init());

        sim_manager.register_do_not_end(0);

        let mut sim_dispatcher_handlers = vec![];
        for sim_dispatcher in sim_dispatchers {
            sim_dispatcher_handlers.push(thread::spawn(move || sim_dispatcher.run()));
        }

        Core {
            ack_channel,
            sim_manager,
            sim_dispatcher_handlers,
            mem_ctl,
            control,
            ir,
            pc_mux,
            pc,
            mar_mux,
            mar,
            mdr,
            alu_mux1,
            alu_mux2,
            alu,
            cmp_mux,
            cmp,
            regfile_mux,
            regfile,
            links,
            commit_file,
            data_out,
            stats,
        }
    }
}

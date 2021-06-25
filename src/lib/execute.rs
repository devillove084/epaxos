#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(missing_docs)]

use std::i8;

use crate::{config::REPLICAS_NUM, logic::{EpaxosLogic, Instance, LogEntry, State}};

pub type Stack = Vec<*mut Instance>;
//static mut ss: Stack = Stack::with_capacity(100);

pub struct Exec {
    r: *mut EpaxosLogic,
}

pub struct SCComponent {
    nodes: Instance,
	color: i8,
}

impl Exec {
    pub fn executeCommand(self,id: &mut EpaxosLogic, log_entry: LogEntry) -> bool {
        unsafe {
            if (*self.r).instance_number == 0 {
                return false;
            }
        }
        
        let inst_status = log_entry.state;
        if inst_status == State::Executed {
            return true;
        } else if inst_status == State::Committed {
            return false;
        }

        if !self.findSCC(id) {
            return false;
        }

        return true;
    }

    pub fn findSCC(&self, root: &mut EpaxosLogic) -> bool {
        let mut index = 1;
        let stack: Vec<Instance> = Vec::new();
        return self.strongconnect(root, &mut index);
    }

    fn strongconnect(&self, v: &mut EpaxosLogic, index: &mut usize) -> bool {
        v.index = *index;
        v.lowlink = *index;
        *index += 1;

        //TODO: Judge stack expand

        let mut q: usize = 0;
        while q <= REPLICAS_NUM {
            q += 1;
            let inst_h = v.cmds.get(*index).unwrap();
            let inst = inst_h.get(&q).unwrap();
            let inst_dep = inst.deps[q];
            //Just a start
            
        }

        true
    }
}
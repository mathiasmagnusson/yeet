use crate::common::{BinaryOperator, UnaryOperator};

#[derive(Debug, Default)]
pub struct Procedure {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Default)]
pub struct Block {
    pub instrs: Vec<Instr>,
}

#[derive(Debug, Clone)]
pub enum Instr {
    DefLocal {
        name: String,
    },
    SymAssign {
        name: String,
        src: Value,
    },
    SymbolVal {
        dest: Value,
        name: String,
    },
    Const {
        dest: Value,
        val: i64,
    },
    BinaryOperation {
        dest: Value,
        lhs: Value,
        rhs: Value,
        operator: BinaryOperator,
    },
    UnaryOperation {
        dest: Value,
        operand: Value,
        operator: UnaryOperator,
    },
    Jump(BlockId),
    Branch {
        cond: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
    Print(Value),
    Return(Value),
}

pub type BlockId = u32;
pub type Value = u32;

<p align="center">
  <img src="zkwasm-bk.png" height="100">
</p>

<p align="center">
  <a href="https://github.com/DelphinusLab/zkWasm/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache 2-blue.svg"></a>
</p>

# Overview：
## Setup input:
wasm code

## Runtime input:
input of wasm function and the top level function must be main.

## Proving target
simulation of wasm execution of target wasm bytecode with particular inputs are correct.

# Static Data：

## Instruction Table
Instruction Table[ITable] is a [FIXED] map reprensents the wasm code image.
**ITable**: address $\mapsto$ opcode

## Initial Memory Table
Initial Memory Table[MTable] is [FIXED] and represents the heap memory of wasm.
**IMTable**: heap memory index $\mapsto$ data[u64] where data is indexed by u64.

## Execution Trace Table:
Exexution Trace Table [ETable] is used to verifying that the execution sequence enforces the semantics of each instruction's bytecode.
**ETable**: eid  execution $\mapsto$ data

## Jump table
(eid, return address, last jump eid）

## Memory Trace Table
((eid+empty, stack + heap) -> memory access log
empty for heap memory initialization
(verifies data access)

## Extra
public inputs for top level functions
(index $\mapsto$ data)

# Prerequisite

## Memory Access Log
Memory access log is used to describe how memory is used.

* MemoryAccessLog = ((Init + Write + Read), (i32/64+f32/64), data) 

## Sujective map

Suppose that A, B are tables, A = ($a_i$) where $a_i$ values of columns $A_i$ and 
B = ($b_i$) where $b_i$ values of columns $B_i$.

Polynomial lookup can prove $\forall {a_i} \in A, f(a_i) \in g(b_i)$ by a map $l$. But can not prove $\forall {b_i} \in B$, there exists ${a_i} \in A$ such that $f(a_i) \in g(b_i)$. To prove $l$ is sujective, we need to either
* find l' from ${b_i}$ to ${a_i}$.
* compare row numbers of $A$ and $B$ and make sure ${a_i}$ are unique and $f(a_i) \neq f(a_j)$ when $a_i \neq a_j$.

# Circuits

## Static Table:
ITable + IMTable

## Dynamic Table:
ETable, MTable, JTable



 
## Instruction Table (ITable)
Instruction encodes the static predefined wasm image and is abstracted as a table of pair **(InstructionAddress, Instruction)**.

* InstructionAddress = (ModuleId, FunctionId, InstructionId)
* Opcode = (OpcodeClass + InnerParameters)
* MemoryAddress = (Stack + Heap, ModuleId, offset)
    * for stack memory, its ModuleId always equals 0.
    * for heap memory, its Module Id start from 1.
    * offset: start from 0.

## Init memory table (IMTable)
Init memory table describes the initial data of heap before execution and is abstracted as a table of pair **IMTable = (HeapMemoryIndex, data)**


## Execution table (ETable):
Execution table represents the execution sequence and it needs to match the semantic of each opcode in the sequence.

**ETable [$T_e$] = (EId, Instruction, SP，restMops, lastJumpEId, RestJops)**

* $eid$ starts from 1 and inc one after each instruction in the ETable.
* $\forall e \in \mathbb{T_e}$ there exists $e'\in \mathbb{T_i}$ such that $e.instruction = e'.instruction$. This constraint proves that each executed bytecode exists in the instruction table.
* $\forall e_k, e_{k+1} \in T_e$
    * $e_k.eid + 1= e_{k+1}.eid$
    * $e1.next(e1.address) = e2.address$.
* lookup(EID, Instruction, SP) --> mtable
    * ($eid$, $i$, $sp$) is unique
    * map from etable to mtable is identical mapping
    * It remains to show two table has same number of memory rw rows:
        * Suppose that $e$ is the last element of $\mathbb{T_e}$, then $e.restMops = 0$.
* sp -> stack memory log [emid] 存在于 mtable
    * mtable log = init + execution


* Example

  | EID | OP | accessType | Address | value|
  | --- |----|-------|---------|------|
  | eid | op | write | address | data |
  | ... | .. | ....  | .....   | ...  |
  | eid | op | read  | address | data |


## Memory Access Table (MTable)
**MTable [$T_m$] = ( eid, emid, address, accessType, type, data)**

* Suppose that $r_k$ and $r_{k+1}$ in **MTable** and $r_{k}.accessType = write$ and $r_{k+1}$.accessType = Read$ Then $r_{k+1}.data = r_k.data$
* Suppose that $r_k$ and $r_{k+1}$ in Mtable, addressCode(r_k) <= addressCode(r_{k+1}) where addressCode(r_k) = address << 2 + emid

# Operations Spec [WIP]
We uses z3 (https://github.com/Z3Prover/z3) to check that all operation are compiled to zkp circuits correctly.
## arithment operation [op_bin]
## bit operation [op_bin_bit]
With a, b in `u4`, we have $2^4\cdot 2^4=256$ for each op in `and, or, xor`.
| a | b | a ^ b| op |
|---|---|------|----|
| 0 | 0 | 0 |   xor |
| 0 | 1 | 1 |   xor |
| 1 | 0 | 1 |   xor|
| 1 | 1 | 0 |   xor|
| ...|...|...| ....|
| 15 | 15| 0 |  xor|


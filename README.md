mtable
encode:
eid(16) | emid(16) | mmid(16) | offset(16) | ltype(1) | atype(2) | vtype(13) | value(64)

encode_stack(eid, sp, atype, vtype, value)
eid << 128 | emid << 112 | 0 << 96 | sp << 80 | 1 << 79 | atype << 77 | vtype << 64 | value

constraints:
1. sorted by address (ltype | mmid | offset)
2. sorted by eid
3. for memory with same address, the first row must be 
init and it should be in init_mtable.
4. for stack with same address, the first row must be write.
5. is_same_address -> curr_atype = read -> last_vtype = curr_vtype
6. is_same_address -> curr_atype = read -> last_value = curr_value


etable constraints:
| eid | moid | fid | bid | iid | mmid | sp | ljid | opcode_bitmaps | opcode | aux |
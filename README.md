mtable constraints:
1. sorted by address (ltype | mmid | offset)
2. sorted by eid

3. for memory with same address, the first row must be 
init and it should be in init_mtable.
4. for stack with same address, the first row must be write.
5. is_same_address -> curr_atype = read -> last_vtype = curr_vtype
6. is_same_address -> curr_atype = read -> last_value = curr_value

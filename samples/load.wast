(module
  (memory 1)
  (data (i32.const 0) "abcdefghijklmnopqrstuvwxyz")

  (func (export "8u_good1") (param $i i32) (result i32)
    (i32.load8_u offset=0 (local.get $i))                   ;; 97 'a'
  )
)

invoke "8u_good1" (i32.const 0)

;; itable
;; (mid, fid, bid, iid, opcode)
;; (1, 1, 0, 1, local.get)
;; (1, 1, 0, 2, i32.load8_u)
;; (1, 1, 0, 3, fend)
;; (0, 0, 0, 1, i32.const 0)
;; (0, 0, 0, 2, invoke "8u_good1")
;; (0, 0, 0, 3, fend)

;; etable
;; (eid, mid, fid, bid, iid, opcode, sp, last_jump_eid)
;; (1, 0, 0, 0, 1, i32.const 0, 4096, 0)
;; (2, 0, 0, 0, 2, invoke "8u_good1", 4095, 0)
;; (3, 1, 1, 0, 1, local.get, 4095, 2)
;; (4, 1, 1, 0, 2, i32.load8_u, 4094, 2)
;; (5, 1, 1, 0, 3, fend, 4095, 2)
;; (6, 0, 0 0, 3, fend, 4095, 0)

;; jtable
;; (last_jump_eid, eid, mid, fid, bid, iid)
;; (0, 2, 0, 0, 0, 2)

;; m_init_table
;; (mid, offset, value)
;; (1, 0, 'a')
;; (1, 1, 'b')
;; ...

;; mtable
;; (eid, mtype, mid, offset, atype, vtype, value)
;; (3, stack, 0, 4094, write, i32, 0)
;; (4, stack, 0, 4094, read, i32, 0)
;; (4, stack, 0, 4094, write, u8, 'a')
;; (5, stack, 0, 4094, read, u8, 'a')
;; (1, stack, 0, 4095, write, i32, 0)
;; (3, stack, 0, 4095, read, i32, 0)
;; (5, stack, 0, 4095, write, u8, 'a')
;; (6, stack, 0, 4095, read, u8, 'a')
;; (6, stack, 0, 4095, write, u8, 'a')
;; (0, memory, 1, 0, init, u8, 'a')
;; (3, memory, 1, 0, read, u8, 'a')
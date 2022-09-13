from z3 import *
from utils import *
from functools import reduce

s = init_z3_solver()

# define spec
lhs, rhs, res = Ints('lhs rhs res')
s.add(is_u64(lhs)),
s.add(is_u64(rhs)),
s.add(is_u64(res)),

wasm_add_u64 = Function('WamsAddU64', IntSort(), IntSort(), IntSort())
s.add(ForAll([lhs, rhs], wasm_add_u64(lhs, rhs) == (lhs + rhs) % U64_MODULUS))

# define var
overflow = Int('overflow')

constrain = Function('Constrain', IntSort(), BoolSort())
constraints = [
    is_bit(overflow),
    fr_sub(fr_sub(fr_add(fr_mul(overflow, U64_MODULUS), res), rhs), lhs) == 0
]

s.add(ForAll([overflow], And(constrain(overflow) == reduce(lambda x, y: And(x, y), constraints))))

s.push()
# Soundness
s.add(And(constrain(overflow), wasm_add_u64(lhs, rhs) != res))

check_res = s.check()
print('--------------Soundness---------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
else:
    print('Verify: Pass')
s.pop()


s.push()
# Completeness
s.add(And(wasm_add_u64(lhs, rhs) == res, ForAll([overflow], Not(constrain(overflow)))))

check_res = s.check()
print('-------------Completeness----------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
else:
    print('Verify: Pass')
s.pop()

from z3 import *
from utils import *
from functools import reduce

s = init_z3_solver()

# define spec
lhs, rhs, res_d, res_m = Ints('lhs rhs res_d res_m')
s.add(is_i64(lhs))
s.add(is_i64(rhs))
s.add(rhs != 0)

wasm_div_u_i64 = Function('WasmDivUI64', IntSort(), IntSort(), IntSort())
s.add(ForAll([lhs, rhs], wasm_div_u_i64(lhs, rhs) == lhs / rhs))
wasm_rem_u_i64 = Function('WasmRemUI64', IntSort(), IntSort(), IntSort())
s.add(ForAll([lhs, rhs], wasm_rem_u_i64(lhs, rhs) == lhs % rhs))

# define var
aux1 = Int('aux1')
aux2 = Int('aux2')
aux3 = Int('aux3')
intermediate = Int('intermediate')

constrain = Function('Constrain', IntSort(), IntSort(),
                     IntSort(), IntSort(), BoolSort())
constraints = [
    is_i64(aux1),
    is_i64(aux2),
    is_i64(aux3),
    is_i64(res_d),
    is_i64(res_m),
    # c.bin.div_u/rem_u
    intermediate == fr_add(fr_mul(rhs, aux1), aux2),
    fr_sub(intermediate, lhs) == 0,
    fr_sub(fr_add(fr_add(aux2, aux3), 1), rhs) == 0,
    fr_sub(res_d, aux1) == 0,
    fr_sub(res_m, aux2) == 0,
]

s.add(ForAll([aux1, aux2, aux3, intermediate], And(constrain(
    aux1, aux2, aux3, intermediate) == reduce(lambda x, y: And(x, y), constraints))))

s.push()
# Soundness
s.add(And(constrain(aux1, aux2, aux3, intermediate),
      Or(wasm_div_u_i64(lhs, rhs) != res_d, wasm_rem_u_i64(lhs, rhs) != res_m)))

check_res = s.check()
print('--------------Soundness---------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()


s.push()
# Completeness
s.add(And(wasm_div_u_i64(lhs, rhs) == res_d, wasm_rem_u_i64(lhs, rhs) == res_m,
          ForAll([aux1, aux2, aux3, intermediate],
                 Not(constrain(aux1, aux2, aux3, intermediate)))))

check_res = s.check()
print('-------------Completeness----------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()

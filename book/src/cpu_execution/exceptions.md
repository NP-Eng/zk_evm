## Exceptions

Sometimes, when executing user code (i.e. contract or transaction code),
the EVM halts exceptionally (i.e. outside of a STOP, a RETURN or a
REVERT). When this happens, the CPU table invokes a special instruction
with a dedicated operation flag `exception`. Exceptions can only happen
in user mode; triggering an exception in kernel mode would make the
proof unverifiable. No matter the exception, the handling is the same:

-- The opcode which would trigger the exception is not executed. The
operation flag set is `exception` instead of the opcode's flag.

-- We push a value to the stack which contains: the current program
counter (to retrieve the faulty opcode), and the current value of
`gas_used`. The program counter is then set to the corresponding
exception handler in the kernel (e.g. `exc_out_of_gas`).

-- The exception handler verifies that the given exception would indeed
be triggered by the faulty opcode. If this is not the case (if the
exception has already happened or if it doesn't happen after executing
the faulty opcode), then the kernel panics: there was an issue during
witness generation.

-- The kernel consumes the remaining gas and returns from the current
context with `success` set to 0 to indicate an execution failure.

Here is the list of the possible exceptions:

1.  Raised when a native instruction (i.e. not a syscall) in user mode
    pushes the amount of gas used over the current gas limit. When this
    happens, the EVM jumps to `exc_out_of_gas`. The kernel then checks
    that the consumed gas is currently below the gas limit, and that
    adding the gas cost of the faulty instruction pushes it over it. If
    the exception is not raised, the prover will panic when returning
    from the execution: the remaining gas is checked to be positive
    after STOP, RETURN or REVERT.

2.  Raised when the read opcode is invalid. It means either that it
    doesn't exist, or that it's a privileged instruction and thus not
    available in user mode. When this happens, the EVM jumps to
    `exc_invalid_opcode`. The kernel then checks that the given opcode
    is indeed invalid. If the exception is not raised, decoding
    constraints ensure no operation flag is set to 1, which would make
    it a padding row. Halting constraints would then make the proof
    unverifiable.

3.  Raised when an instruction which pops from the stack is called when
    the stack doesn't have enough elements. When this happens, the EVM
    jumps to `exc_stack_overflow`. The kernel then checks that the
    current stack length is smaller than the minimum stack length
    required by the faulty opcode. If the exception is not raised, the
    popping memory operation's address offset would underflow, and the
    Memory range check would require the Memory trace to be too large
    ($>2^{32}$).

4.  Raised when the program counter jumps to an invalid location (i.e.
    not a JUMPDEST). When this happens, the EVM jumps to
    `exc_invalid_jump_destination`. The kernel then checks that the
    opcode is a JUMP, and that the destination is not a JUMPDEST by
    checking the JUMPDEST segment. If the exception is not raised,
    jumping constraints will fail the proof.

5.  Same as the above, for JUMPI.

6.  Raised when a pushing instruction in user mode pushes the stack
    over 1024. When this happens, the EVM jumps to `exc_stack_overflow`.
    The kernel then checks that the current stack length is exactly
    equal to 1024 (since an instruction can only push once at most), and
    that the faulty instruction is pushing. If the exception is not
    raised, stack constraints ensure that a stack length of 1025 in user
    mode will fail the proof.

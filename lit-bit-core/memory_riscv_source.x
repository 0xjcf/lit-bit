/* memory.x */
MEMORY
{
  /* QEMU 'virt' machine RAM starts at 0x80000000. */
  RAM : ORIGIN = 0x80000000, LENGTH = 16M /* Keep reduced RAM size for now */
}

/* All sections will be placed in RAM by the main linker script (from riscv-rt) */
REGION_ALIAS("REGION_TEXT", RAM);
REGION_ALIAS("REGION_RODATA", RAM);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

/* Define symbols needed by riscv-rt's linker script. 
   These values are suitable for a single-core system with no pre-allocated heap. */
_heap_size = 0K;
_hart_stack_size = 8K; /* Stack per hart. Increased from 2K for safety. */
_max_hart_id = 0;      /* Max hart ID, 0 for single-core. */

/* _stack_start is typically calculated by the main linker script as:
   _stack_start = ORIGIN(REGION_STACK) + LENGTH(REGION_STACK);
   If REGION_STACK is RAM, it will be the top of RAM. Let's define it explicitly. */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

/* _stext (start of text) is also typically handled by the main linker script,
   placing it at ORIGIN(REGION_TEXT). */

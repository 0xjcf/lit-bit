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

/* Let riscv-rt's link.x provide defaults for:
   _heap_size (defaults to 0 if not set, or based on REGION_HEAP)
   _hart_stack_size (defaults to 2K)
   _max_hart_id (defaults to 0 for single-core)
   _stack_start (defaults to top of REGION_STACK)
   based on the MEMORY regions defined above.
*/

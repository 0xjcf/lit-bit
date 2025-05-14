/* memory_cortex_m.x - Linker script for generic thumbv7m target */
MEMORY
{
  /* Example values: Adjust to your target device or typical QEMU setup for Cortex-M */
  FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 256K /* Example: 256KB Flash */
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 64K  /* Example: 64KB RAM */
}

/* Define symbols that cortex-m-rt might expect or use */
/* _stack_start = ORIGIN(RAM) + LENGTH(RAM); /* Typically defined by cortex-m-rt itself */
/* _heap_start = something; */
/* _heap_end = something; */ 
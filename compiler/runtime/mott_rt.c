/* Mott runtime library
 *
 * Minimal helpers linked into every compiled Mott program:
 *   - string representation (length + bytes, UTF-8)
 *   - string interpolation (formatting yazde arguments)
 *   - yazde() implementations per type
 *
 * Filled in once the C backend starts emitting code.
 */

#include <stdint.h>
#include <stdio.h>

import * as MathLib from "./math";

export function compute(x: number): number {
  return MathLib.add(MathLib.multiply(x, 2), MathLib.PI);
}

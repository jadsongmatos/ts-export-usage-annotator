import { greet, VERSION, User } from "./lib";
import mainFn from "./lib";

export function run(): void {
  const user: User = { id: 1, name: "test" };
  console.log(greet(user.name), VERSION);
  mainFn();
}

The maximum bounds for patterns are calculated using this logic.

```ts
// Let's assume our server has maximum 1.5GB of memory pesimistically
let totalMem = 1_610_612_736;

let patternLen = 32;
let bytesPerChar = 4;
let bytesPerPattern = patternLen * bytesPerChar;
let patterns = 100;

let bytesPerChat = patterns * bytesPerPattern;
let maxChats = totalMem / bytesPerChat;

console.log({ maxChats, bytesPerChat });
// {maxChats: 67108.864, bytesPerChat: 24000}
```

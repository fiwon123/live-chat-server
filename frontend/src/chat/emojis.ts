// Curated built-in emoji set for the composer picker — no dependency.

export interface EmojiEntry {
  emoji: string;
  name: string;
}

export const EMOJIS: EmojiEntry[] = [
  { emoji: "😀", name: "grinning" },
  { emoji: "😄", name: "smile" },
  { emoji: "😁", name: "beaming" },
  { emoji: "🤣", name: "rofl" },
  { emoji: "😅", name: "sweat smile" },
  { emoji: "🥲", name: "smile tear" },
  { emoji: "😊", name: "blush" },
  { emoji: "😉", name: "wink" },
  { emoji: "😍", name: "heart eyes" },
  { emoji: "😘", name: "kiss" },
  { emoji: "🤩", name: "star struck" },
  { emoji: "😎", name: "cool" },
  { emoji: "🤔", name: "thinking" },
  { emoji: "🤗", name: "hug" },
  { emoji: "🙃", name: "upside down" },
  { emoji: "😌", name: "relieved" },
  { emoji: "😴", name: "sleeping" },
  { emoji: "🥳", name: "party" },
  { emoji: "😭", name: "crying" },
  { emoji: "😢", name: "sad" },
  { emoji: "😤", name: "triumph" },
  { emoji: "😡", name: "angry" },
  { emoji: "🤯", name: "mind blown" },
  { emoji: "🙏", name: "thanks" },
  { emoji: "👏", name: "clap" },
  { emoji: "👍", name: "thumbs up" },
  { emoji: "👎", name: "thumbs down" },
  { emoji: "👋", name: "wave" },
  { emoji: "🤝", name: "handshake" },
  { emoji: "💪", name: "flex" },
  { emoji: "🔥", name: "fire" },
  { emoji: "✨", name: "sparkles" },
  { emoji: "🎉", name: "party popper" },
  { emoji: "❤️", name: "heart" },
  { emoji: "💯", name: "hundred" },
  { emoji: "🚀", name: "rocket" },
  { emoji: "⭐", name: "star" },
  { emoji: "✅", name: "check mark" },
  { emoji: "👀", name: "eyes" },
  { emoji: "💡", name: "idea" },
];

export const EMOJI_BY_NAME = new Map(EMOJIS.map((entry) => [entry.name, entry.emoji]));

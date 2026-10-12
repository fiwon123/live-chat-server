import { avatarColor, initialsOf } from "../chat/avatar";

export function Avatar({ name, size = 32 }: { name: string; size?: number }) {
  return (
    <span
      className="avatar"
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.4),
        background: avatarColor(name),
      }}
      aria-hidden="true"
    >
      {initialsOf(name)}
    </span>
  );
}

// Label + bendera bahasa ala mobile (chip English dkk).
export function langFlag(code?: string | null): string {
  switch ((code ?? "").toLowerCase()) {
    case "en":
      return "🇬🇧";
    case "ja":
      return "🇯🇵";
    case "zh":
      return "🇨🇳";
    case "id":
      return "🇮🇩";
    default:
      return "🌐";
  }
}

export function langLabel(code?: string | null): string {
  switch ((code ?? "").toLowerCase()) {
    case "en":
      return "English";
    case "ja":
      return "Japanese";
    case "zh":
      return "Chinese";
    case "id":
      return "Indonesia";
    default:
      return (code ?? "").toUpperCase() || "?";
  }
}

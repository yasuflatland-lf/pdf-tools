import type { ReactElement } from "react";
import type { SourceStatusDto } from "../bindings/SourceStatusDto";

function errorMessage(status: SourceStatusDto): string | null {
  if (status.kind === "ready") {
    return null;
  }
  if (status.kind === "encrypted") {
    return "パスワードで保護されているため結合できません";
  }
  return `読み込めないため結合できません: ${status.reason}`;
}

export function ErrorBadge({ status }: { status: SourceStatusDto }): ReactElement | null {
  const message = errorMessage(status);

  if (message === null) {
    return null;
  }

  return (
    <p className="mt-2 rounded-md border border-amber-700/60 bg-amber-950/50 px-2 py-1 text-xs text-amber-100">
      {message}
    </p>
  );
}

import type { ReactElement } from "react";
import type { SourceStatusDto } from "../bindings/SourceStatusDto";
import { Notice } from "./card/Notice";

function statusMessage(status: SourceStatusDto): string | null {
  if (status.kind === "ready") {
    return null;
  }
  if (status.kind === "encrypted") {
    return "Password protected";
  }
  switch (status.reason) {
    case "unsupportedFormat":
      return "This file format is not supported";
    case "damaged":
      return "This file is damaged and could not be read";
    case "missing":
      return "This file could not be found";
    case "engineUnavailable":
      return "The PDF engine is unavailable";
  }
}

export function ErrorBadge({ status }: { status: SourceStatusDto }): ReactElement | null {
  const message = statusMessage(status);

  if (message === null) {
    return null;
  }

  return <Notice>{message}</Notice>;
}

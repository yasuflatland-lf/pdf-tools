import type { CardViewProps } from "./card/CardProps";
import { ToggleButton } from "./card/ToggleButton";
import { PageCard } from "./PageCard";

type Props = CardViewProps;

export function GroupCard({ onToggle, ...pageCardProps }: Props) {
  const { collapsed, fileName } = pageCardProps;

  return (
    // Both toggles sit outside `PageCard`, so the pointer can rest on one of
    // them without entering the hover marker inside the card. Marking the
    // element that holds the card and the toggle together reveals the rotate
    // buttons wherever on the card the pointer is -- `group-hover` matches any
    // marked ancestor, so the inner marker keeps working for a card with no
    // toggle at all.
    <div className="group relative">
      <PageCard {...pageCardProps} />
      {onToggle && (
        // A badge rather than a blanket over the preview: the preview is most
        // of the card, and a control covering it would take every click meant
        // to select the card -- including the Shift-click that ends a range.
        <ToggleButton
          collapsed={collapsed}
          fileName={fileName}
          onToggle={onToggle}
          className="absolute top-2 left-2"
        />
      )}
    </div>
  );
}

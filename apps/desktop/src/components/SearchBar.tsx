import Button from "./ui/Button";
import Input from "./ui/Input";

interface Props {
  value: string;
  onChange: (value: string) => void;
  onSave: () => void;
  canSave: boolean;
}

export default function SearchBar({ value, onChange, onSave, canSave }: Props) {
  return (
    <div className="flex items-center gap-3">
      <Input
        type="text"
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="resume  type:pdf  size>10mb  age<1y  in:downloads"
        className="flex-1 py-3"
      />
      <Button onClick={onSave} disabled={!canSave} className="shrink-0 py-3">
        Save
      </Button>
    </div>
  );
}

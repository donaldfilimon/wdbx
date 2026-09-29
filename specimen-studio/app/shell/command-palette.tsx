import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from '@/components/ui/command';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog';
import type { ShellCommand } from './commands';

const GROUPS: ShellCommand['group'][] = [
  'Go to',
  'Open to the side',
  'Specimen',
  'View',
];

/**
 * ⌘K palette. Composed here rather than with the kit's CommandDialog, which
 * renders its children without a cmdk <Command> root and places the title
 * outside the dialog popup (no accessible name).
 */
export function CommandPalette({
  open,
  onOpenChange,
  commands,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  commands: ShellCommand[];
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="command-palette top-1/3 translate-y-0 overflow-hidden p-0"
        showCloseButton={false}
      >
        <DialogTitle className="sr-only">Command palette</DialogTitle>
        <DialogDescription className="sr-only">
          Go to a view, open one to the side, or run a specimen action.
        </DialogDescription>
        <Command>
          <CommandInput placeholder="Type a command or view" />
          <CommandList>
            <CommandEmpty>No matching command.</CommandEmpty>
            {GROUPS.map((group) => {
              const items = commands.filter((c) => c.group === group);
              if (!items.length) return null;
              return (
                <CommandGroup key={group} heading={group}>
                  {items.map((c) => (
                    <CommandItem
                      key={c.id}
                      value={c.label}
                      onSelect={() => {
                        onOpenChange(false);
                        c.run();
                      }}
                    >
                      {c.label}
                      {c.shortcut && (
                        <CommandShortcut>{c.shortcut}</CommandShortcut>
                      )}
                    </CommandItem>
                  ))}
                </CommandGroup>
              );
            })}
          </CommandList>
        </Command>
      </DialogContent>
    </Dialog>
  );
}

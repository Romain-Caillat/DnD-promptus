import { Button as ButtonPrimitive } from "@base-ui/react/button"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

/*
 * shadcn's button, redrawn as a key of the pixel menu (board « Pistes UI »,
 * track A, ui/adopt-pixel-menu): stepped corners, flat relief, Silkscreen
 * upper case, the RPG cursor on keyboard focus. The look lives in the
 * `pixel-*` utilities of styles/tokens.css; this file only picks them.
 */
const buttonVariants = cva(
  "group/button pixel-key pixel-cursor inline-flex shrink-0 items-center justify-center whitespace-nowrap outline-none select-none disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "",
        outline: "pixel-key-dark",
        secondary: "pixel-key-dark",
        ghost: "pixel-key-flat",
        destructive: "pixel-key-danger",
        link: "pixel-key-flat underline-offset-4 hover:underline",
      },
      size: {
        default: "min-h-10 gap-2 pr-3.5 pb-1.5 text-label [--px:2px]",
        xs: "min-h-8 gap-1 pr-2 pb-1 text-label [--px:2px] [&_svg:not([class*='size-'])]:size-3",
        sm: "min-h-9 gap-1.5 pr-2.5 pb-1 text-label [--px:2px] [&_svg:not([class*='size-'])]:size-3.5",
        lg: "min-h-12 gap-2.5 pr-4 pb-2 text-pixel",
        icon: "size-10 pl-0! before:hidden [--px:2px]",
        "icon-xs": "size-8 pl-0! before:hidden [--px:2px] [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-9 pl-0! before:hidden [--px:2px]",
        "icon-lg": "size-12 pl-0! before:hidden",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }

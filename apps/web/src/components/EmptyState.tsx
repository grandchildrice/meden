import { Typography } from "@mui/material";
import type { ReactNode } from "react";
export function EmptyState({
  title,
  detail,
  icon,
  children,
}: {
  title: string;
  detail: string;
  icon?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="empty-state">
      {icon && <div className="empty-icon">{icon}</div>}
      <Typography variant="h2">{title}</Typography>
      <Typography color="text.secondary">{detail}</Typography>
      {children && <div className="empty-actions">{children}</div>}
    </div>
  );
}

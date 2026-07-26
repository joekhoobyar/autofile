import { forwardRef } from 'react';
import { Toast, type ToastProps } from 'primereact/toast';

export const AppToast = forwardRef<Toast, ToastProps>(function AppToast(props, ref) {
  return <Toast ref={ref} position="top-center" {...props} />;
});

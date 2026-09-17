import type { Appointment } from './api';

export type AppointmentDisplayStatus =
  | 'upcoming'
  | 'due_now'
  | 'overdue'
  | 'arrived'
  | 'in_progress'
  | 'completed'
  | 'cancelled'
  | 'no_show';

export const appointmentDisplayLabels: Record<AppointmentDisplayStatus, string> = {
  upcoming: 'قادم',
  due_now: 'موعد الآن',
  overdue: 'فائت',
  arrived: 'وصل',
  in_progress: 'قيد الخدمة',
  completed: 'مكتمل',
  cancelled: 'ملغي',
  no_show: 'لم يحضر',
};

export function appointmentDisplayStatus(
  appointment: Pick<Appointment, 'status' | 'startsAt' | 'endsAt'>,
  now = new Date(),
): AppointmentDisplayStatus {
  if (appointment.status !== 'scheduled') {
    if (
      appointment.status === 'arrived' ||
      appointment.status === 'in_progress' ||
      appointment.status === 'completed' ||
      appointment.status === 'cancelled' ||
      appointment.status === 'no_show'
    ) {
      return appointment.status;
    }
    return 'upcoming';
  }

  const starts = new Date(appointment.startsAt).getTime();
  const ends = new Date(appointment.endsAt).getTime();
  const current = now.getTime();
  if (Number.isFinite(ends) && ends < current) return 'overdue';
  if (Number.isFinite(starts) && starts <= current && (!Number.isFinite(ends) || current <= ends)) {
    return 'due_now';
  }
  return 'upcoming';
}

export function appointmentDisplayLabel(
  appointment: Pick<Appointment, 'status' | 'startsAt' | 'endsAt'>,
  now = new Date(),
) {
  return appointmentDisplayLabels[appointmentDisplayStatus(appointment, now)];
}

export function appointmentDisplayClass(
  appointment: Pick<Appointment, 'status' | 'startsAt' | 'endsAt'>,
  now = new Date(),
) {
  return `status status-${appointmentDisplayStatus(appointment, now)}`;
}

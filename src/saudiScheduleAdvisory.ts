import type { AppointmentInput, ClosureDate, SchedulingSettings } from './api';

export type ScheduleAdvisorySeverity = 'block' | 'warning';
export type ScheduleAdvisory = {
  code: string;
  severity: ScheduleAdvisorySeverity;
  message: string;
};

const toMinutes = (value?: string) => {
  if (!value) return null;
  const match = value.match(/^(\d{2}):(\d{2})$/);
  if (!match) return null;
  return Number(match[1]) * 60 + Number(match[2]);
};

const parseLocalDateTime = (value: string) => {
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/);
  if (!match) return null;
  const [, year, month, day, hour, minute] = match.map(Number);
  const date = new Date(year, month - 1, day, hour, minute, 0, 0);
  if (
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day ||
    date.getHours() !== hour ||
    date.getMinutes() !== minute
  ) return null;
  return date;
};

const religiousDateParts = (date: Date) => {
  for (const calendar of ['islamic-umalqura', 'islamic']) {
    try {
      const formatter = new Intl.DateTimeFormat(`en-US-u-ca-${calendar}`, {
        day: 'numeric',
        month: 'numeric',
      });
      const parts = formatter.formatToParts(date);
      const day = Number(parts.find((part) => part.type === 'day')?.value);
      const month = Number(parts.find((part) => part.type === 'month')?.value);
      if (Number.isFinite(day) && Number.isFinite(month)) return { day, month };
    } catch {
      // Chromium builds that do not expose the preferred calendar simply skip this advisory.
    }
  }
  return null;
};

export function getSaudiScheduleAdvisories(
  appointment: Pick<AppointmentInput, 'startsAt' | 'durationMinutes'>,
  settings: SchedulingSettings | null,
  closures: ClosureDate[],
): ScheduleAdvisory[] {
  const start = parseLocalDateTime(appointment.startsAt);
  if (!start) return [{ code: 'invalid-date', severity: 'block', message: 'تاريخ أو وقت الموعد غير صالح.' }];

  const advisories: ScheduleAdvisory[] = [];
  const dateKey = appointment.startsAt.slice(0, 10);
  const weekday = start.getDay();
  const startMinutes = start.getHours() * 60 + start.getMinutes();
  const endMinutes = startMinutes + appointment.durationMinutes;

  if (weekday === 5 || weekday === 6) {
    advisories.push({
      code: 'weekend',
      severity: 'block',
      message: 'اليوم المحدد عطلة أسبوعية (الجمعة أو السبت) ولا يقبل الحجز.',
    });
  }

  const closure = closures.find((item) => item.closureDate === dateKey);
  if (closure) {
    advisories.push({
      code: 'closure',
      severity: 'block',
      message: `اليوم محدد كيوم إغلاق${closure.reason ? `: ${closure.reason}` : ''}.`,
    });
  }

  if (settings) {
    const workStart = toMinutes(settings.workStart);
    const workEnd = toMinutes(settings.workEnd);
    if (workStart !== null && workEnd !== null && (startMinutes < workStart || endMinutes > workEnd)) {
      advisories.push({
        code: 'outside-hours',
        severity: 'block',
        message: `الموعد خارج وقت الدوام المعتمد (${settings.workStart}–${settings.workEnd}).`,
      });
    }
    const breakStart = toMinutes(settings.breakStart);
    const breakEnd = toMinutes(settings.breakEnd);
    if (breakStart !== null && breakEnd !== null && startMinutes < breakEnd && endMinutes > breakStart) {
      advisories.push({
        code: 'break-time',
        severity: 'block',
        message: `الموعد يتداخل مع فترة الاستراحة (${settings.breakStart}–${settings.breakEnd}).`,
      });
    }
  }

  const month = start.getMonth() + 1;
  const day = start.getDate();
  if (month === 2 && day === 22) {
    advisories.push({
      code: 'founding-day',
      severity: 'warning',
      message: 'التاريخ يوافق يوم التأسيس. تحقق من اعتماد الإجازة في جهة العمل وأضف يوم الإغلاق عند الحاجة.',
    });
  }
  if (month === 9 && day === 23) {
    advisories.push({
      code: 'national-day',
      severity: 'warning',
      message: 'التاريخ يوافق اليوم الوطني. تحقق من اعتماد الإجازة في جهة العمل وأضف يوم الإغلاق عند الحاجة.',
    });
  }

  const religious = religiousDateParts(start);
  if (religious?.month === 9) {
    advisories.push({
      code: 'ramadan',
      severity: 'warning',
      message: 'التاريخ يقع في رمضان؛ تحقق من ساعات الدوام الرمضانية المعتمدة قبل تثبيت الموعد.',
    });
  }
  if (religious?.month === 10 && religious.day >= 1 && religious.day <= 4) {
    advisories.push({
      code: 'eid-fitr',
      severity: 'warning',
      message: 'التاريخ يقع ضمن فترة عيد الفطر؛ تحقق من أيام الإجازة الرسمية المعتمدة في المنشأة.',
    });
  }
  if (religious?.month === 12 && religious.day >= 8 && religious.day <= 13) {
    advisories.push({
      code: 'hajj-eid-adha',
      severity: 'warning',
      message: 'التاريخ يقع ضمن فترة الحج/عيد الأضحى؛ تحقق من أيام الإجازة والدوام المعتمدة قبل تثبيت الموعد.',
    });
  }

  return advisories;
}

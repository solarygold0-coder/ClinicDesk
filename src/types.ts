export type Patient={id:number;fileNo:number;fullName:string;nationalId?:string;phone?:string};
export type AppointmentStatus='scheduled'|'arrived'|'in_progress'|'completed'|'cancelled'|'no_show';
export type Appointment={id:number;patientId:number;startsAt:string;endsAt:string;status:AppointmentStatus};

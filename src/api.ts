import { invoke } from '@tauri-apps/api/core';

let actorToken: string | null = null;
export const setActorToken = (token: string | null) => { actorToken = token; };
const authed = <T>(command: string, args: Record<string, unknown> = {}) =>
  invoke<T>(command, { ...args, actorToken: actorToken ?? undefined });

export type Patient={id:number;fileNo:number;nationalId?:string;fullName:string;phone?:string;birthDate?:string;sex?:string;medicalSummary?:string;chronicDiseases?:string;allergies?:string;notes?:string};
export type PatientInput={nationalId?:string;fullName:string;phone?:string;birthDate?:string;sex?:string;medicalSummary?:string;chronicDiseases?:string;allergies?:string;notes?:string};
export type Clinic={id:number;name:string;phone?:string;address?:string;isActive:boolean};
export type ClinicInput={name:string;phone?:string;address?:string};
export type Doctor={id:number;clinicId?:number;name:string;specialty?:string;phone?:string;isActive:boolean};
export type DoctorInput={clinicId?:number|null;name:string;specialty?:string;phone?:string};
export type Appointment={id:number;patientId:number;fileNo:number;patientName:string;clinicId?:number;clinicName?:string;doctorId?:number;doctorName?:string;startsAt:string;endsAt:string;status:string;notes?:string};
export type AppointmentInput={patientFileNo:number;clinicId?:number;doctorId?:number;startsAt:string;durationMinutes:number;notes?:string};
export type ProviderUnavailabilityReason='leave'|'sudden_absence'|'assignment_meeting'|'emergency'|'other';
export type ProviderUnavailabilityInput={doctorId:number;unavailableFrom:string;unavailableTo:string;reasonCode:ProviderUnavailabilityReason;reasonNote?:string};
export type ProviderUnavailabilityEvent={id:number;doctorId:number;unavailableFrom:string;unavailableTo:string;reasonCode:ProviderUnavailabilityReason;reasonNote?:string;createdAt:string;resolvedAt?:string};
export type AffectedAppointment={appointmentId:number;patientId:number;fileNo:number;patientName:string;clinicId?:number;doctorId?:number;startsAt:string;endsAt:string;status:string};
export type ProviderResolutionAction='transfer'|'reschedule'|'cancel';
export type ProviderResolutionInput={appointmentId:number;actionType:ProviderResolutionAction;replacementDoctorId?:number;newStartsAt?:string};
export type Attachment={id:number;patientId:number;storedName:string;originalName:string;displayName:string;category?:string;mimeType?:string;sizeBytes:number;sha256:string;createdAt:string;deletedAt?:string;deletedReason?:string};
export type VisitTrackingInput={visitType:'new'|'follow_up'|'renewal';visitStage:'scheduled'|'reception'|'with_doctor'|'completed';followUpAt?:string};
export type FollowUpVisit={appointmentId:number;patientId:number;fileNo:number;patientName:string;followUpAt:string;visitType:string};
export type SchedulingSettings={workStart:string;workEnd:string;breakStart?:string;breakEnd?:string;slotMinutes:number};
export type ClosureDate={id:number;closureDate:string;reason?:string};
export type RoleType='ordinary_employee'|'general_manager'|'deputy_manager'|'doctor'|'specialist';
export type AccountStatus='ACTIVE'|'SUSPENDED'|'CLOSED_INACTIVITY'|'ENDED_SERVICE'|'RETIRED';
export type UserSummary={id:number;username:string;displayName:string;employeeCode:string;roleType:RoleType;accountStatus:AccountStatus;mustChangePassword:boolean};
export type AuthSession={token:string;expiresAt:string;user:UserSummary};
export type AuditEntry={id:number;eventType:string;entityType:string;entityId?:number;detailsJson?:string;actorUserId?:number;actorDisplayName?:string;actorEmployeeCode?:string;actorSessionId?:string;beforeJson?:string;afterJson?:string;reason?:string;createdAt:string};

export const api={
login:(username:string,password:string)=>invoke<AuthSession>('auth_login',{username,password}),
validateSession:(token:string)=>invoke<UserSummary>('auth_validate_session',{token}),
changePassword:(token:string,currentPassword:string,newPassword:string)=>invoke<UserSummary>('auth_change_password',{actorToken:token,currentPassword,newPassword}),
logout:(token:string)=>invoke<void>('auth_logout',{token}),
users:()=>authed<UserSummary[]>('user_list'),
createUser:(username:string,displayName:string,password:string,roleType:RoleType)=>authed<UserSummary>('user_create',{username,displayName,password,roleType}),
updateUser:(id:number,username:string,displayName:string,roleType:RoleType)=>authed<UserSummary>('user_update',{id,username,displayName,roleType}),
setUserStatus:(id:number,status:AccountStatus,reason?:string)=>authed<void>('user_set_status',{id,status,reason}),
resetUserPassword:(id:number,temporaryPassword:string)=>authed<void>('user_reset_password',{id,temporaryPassword}),
deputyRestorePermission:()=>authed<boolean>('deputy_restore_permission_get'),
setDeputyRestorePermission:(enabled:boolean)=>authed<void>('deputy_restore_permission_set',{enabled}),
audit:(limit=500)=>authed<AuditEntry[]>('audit_recent',{limit}),
patients:(query='')=>authed<Patient[]>('patient_list',{query,limit:50}),patientCount:()=>authed<number>('patient_count'),patientByFileNo:(fileNo:number)=>authed<Patient|null>('patient_by_file_no',{fileNo}),inactivePatients:(years=10,limit=100)=>authed<Patient[]>('patient_inactive',{years,limit}),patientAppointments:(patientId:number,limit=30)=>authed<Appointment[]>('patient_appointments',{patientId,limit}),patientFutureAppointmentCount:(id:number)=>authed<number>('patient_future_appointment_count',{id}),createPatient:(input:PatientInput)=>authed<Patient>('patient_create',{input}),updatePatient:(id:number,input:PatientInput)=>authed<Patient>('patient_update',{id,input}),deletePatient:(id:number)=>authed<void>('patient_delete',{id}),
attachments:(patientId:number)=>authed<Attachment[]>('attachment_list',{patientId}),archivedAttachments:(patientId:number)=>authed<Attachment[]>('attachment_archived_list',{patientId}),importAttachment:(patientId:number,sourcePath:string,displayName?:string,category?:string)=>authed<number>('attachment_import',{patientId,sourcePath,displayName,category}),openAttachment:(id:number)=>authed<void>('attachment_open',{id}),archiveAttachment:(id:number,reason:string)=>authed<void>('attachment_archive',{id,reason}),restoreAttachment:(id:number,reason:string)=>authed<void>('attachment_restore',{id,reason}),
clinics:()=>authed<Clinic[]>('clinic_list'),createClinic:(input:ClinicInput)=>authed<Clinic>('clinic_create',{input}),updateClinic:(id:number,input:ClinicInput)=>authed<Clinic>('clinic_update',{id,input}),deleteClinic:(id:number)=>authed<void>('clinic_delete',{id}),doctors:()=>authed<Doctor[]>('doctor_list'),createDoctor:(input:DoctorInput)=>authed<Doctor>('doctor_create',{input}),updateDoctor:(id:number,input:DoctorInput)=>authed<Doctor>('doctor_update',{id,input}),deleteDoctor:(id:number)=>authed<void>('doctor_delete',{id}),appointments:(from:string,to:string)=>authed<Appointment[]>('appointment_list',{from,to}),missedAppointments:(to:string)=>authed<Appointment[]>('appointment_missed_history',{to}),upcomingAppointments:(from:string)=>authed<Appointment[]>('appointment_upcoming_all',{from}),createAppointment:(input:AppointmentInput)=>authed<Appointment>('appointment_create',{input}),updateAppointment:(id:number,input:AppointmentInput)=>authed<Appointment>('appointment_update',{id,input}),appointmentStatus:(id:number,status:string)=>authed<void>('appointment_status',{id,status}),providerUnavailabilityOpen:()=>authed<ProviderUnavailabilityEvent[]>('provider_unavailability_list_open'),createProviderUnavailability:(input:ProviderUnavailabilityInput)=>authed<ProviderUnavailabilityEvent>('provider_unavailability_create',{input}),providerUnavailabilityAffected:(eventId:number)=>authed<AffectedAppointment[]>('provider_unavailability_affected',{eventId}),resolveProviderUnavailability:(eventId:number,resolutions:ProviderResolutionInput[])=>authed<void>('provider_unavailability_resolve_many',{eventId,resolutions}),visitTracking:(id:number)=>authed<VisitTrackingInput>('visit_tracking_get',{id}),updateVisitTracking:(id:number,input:VisitTrackingInput)=>authed<void>('visit_tracking_update',{id,input}),followUps:(from:string,to:string)=>authed<FollowUpVisit[]>('visit_follow_ups',{from,to}),schedulingSettings:()=>authed<SchedulingSettings>('scheduling_settings_get'),updateSchedulingSettings:(input:SchedulingSettings)=>authed<SchedulingSettings>('scheduling_settings_update',{input}),closures:()=>authed<ClosureDate[]>('closure_list'),createClosure:(input:{closureDate:string;reason?:string})=>authed<void>('closure_create',{input}),deleteClosure:(id:number)=>authed<void>('closure_delete',{id}),createBackup:(destinationPath:string)=>authed<string>('backup_create',{destinationPath}),restoreBackup:(sourcePath:string)=>authed<string>('backup_restore',{sourcePath})};

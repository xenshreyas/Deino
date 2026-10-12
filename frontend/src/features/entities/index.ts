/**
 * People and companies (ORG-04): the People and Companies lists and pages,
 * the person hover card, the meeting header's attendee chip, and the slots
 * AI-02 (scoped chat) and CAP-04 (voice) fill. Other code imports from here
 * only.
 */
export { companiesApi, peopleApi } from './api';
export { CompaniesScreen } from './components/companies-screen';
export { CompanyScreen } from './components/company-screen';
export { MeetingAttendees, type MeetingAttendeesProps } from './components/meeting-attendees';
export { MentionHoverZone } from './components/mention-hover-zone';
export { PeopleScreen } from './components/people-screen';
export { PersonHoverCard, type PersonHoverCardProps } from './components/person-hover-card';
export { PersonScreen } from './components/person-screen';
export { entitiesCopy } from './copy';
export {
  companiesListQuery,
  companyPageQuery,
  entityKeys,
  peopleListQuery,
  personPageQuery,
} from './queries';
export { type AskScope, type AskSlotProps, ENTITY_SLOTS, type VoiceSlotProps } from './slots';
export { asCompanyId, asPersonId, type CompanyId, type PersonId } from './types';

import { FolderOpen, BriefcaseBusiness, Code, BookOpen, GraduationCap, Brain, ChartNoAxesCombined, Target, FlaskConical, Heart, Dumbbell, Music, Camera, Palette, Gamepad2, Globe, Plane, House, Wrench, Lightbulb, Rocket, Sprout, Coffee, Database } from '@lucide/svelte';
import techIcons from './tech-icons.json';

export const generalIcons = [
  { id: '', label: 'Default folder', keywords: 'reset none folder', component: FolderOpen },
  { id: 'lucide:briefcase', label: 'Career', keywords: 'job work interview business', component: BriefcaseBusiness },
  { id: 'lucide:code', label: 'Code', keywords: 'programming development', component: Code },
  { id: 'lucide:book', label: 'Reading', keywords: 'book study learn', component: BookOpen },
  { id: 'lucide:graduation', label: 'Learning', keywords: 'course study school', component: GraduationCap },
  { id: 'lucide:brain', label: 'Mind', keywords: 'mental mindfulness thinking', component: Brain },
  { id: 'lucide:chart', label: 'Trading', keywords: 'finance stocks markets chart', component: ChartNoAxesCombined },
  { id: 'lucide:target', label: 'Goal', keywords: 'target focus', component: Target },
  { id: 'lucide:flask', label: 'Research', keywords: 'science experiment', component: FlaskConical },
  { id: 'lucide:heart', label: 'Health', keywords: 'heart wellbeing', component: Heart },
  { id: 'lucide:dumbbell', label: 'Fitness', keywords: 'exercise sport gym', component: Dumbbell },
  { id: 'lucide:music', label: 'Music', keywords: 'audio practice', component: Music },
  { id: 'lucide:camera', label: 'Video', keywords: 'photo camera photography', component: Camera },
  { id: 'lucide:palette', label: 'Design', keywords: 'art drawing creative', component: Palette },
  { id: 'lucide:gamepad', label: 'Games', keywords: 'gaming play', component: Gamepad2 },
  { id: 'lucide:globe', label: 'Explore', keywords: 'world languages web', component: Globe },
  { id: 'lucide:plane', label: 'Travel', keywords: 'trip vacation', component: Plane },
  { id: 'lucide:house', label: 'Home', keywords: 'house personal', component: House },
  { id: 'lucide:wrench', label: 'Build', keywords: 'tools repair project', component: Wrench },
  { id: 'lucide:lightbulb', label: 'Ideas', keywords: 'inspiration thinking', component: Lightbulb },
  { id: 'lucide:rocket', label: 'Launch', keywords: 'startup project', component: Rocket },
  { id: 'lucide:sprout', label: 'Growth', keywords: 'habits plants nature', component: Sprout },
  { id: 'lucide:coffee', label: 'Coffee', keywords: 'break hobby', component: Coffee },
  { id: 'lucide:database', label: 'Data', keywords: 'database sql analytics', component: Database },
].map(icon => ({ ...icon, category: 'General' }));

export const questIcons = [...generalIcons, ...techIcons];
export function questIconLabel(id: string) {
  return questIcons.find(icon => icon.id === id)?.label ?? 'Default folder';
}
export { techIcons };

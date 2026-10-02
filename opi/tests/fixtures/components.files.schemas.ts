/** @description A reusable component */
export type Component = {
	type: "component";
	/** @description Server-assigned id */
	readonly id?: string;
	component: Record<string, unknown>;
	tags?: string[] | null;
};
export type Error2 = {
	type: "error";
	message: string;
};
export type Mixed = Array<string | 1 | -2 | null>;

/** @summary Fetch a component */
export type GetComponentsByName = {
	method: "get";
	path: "/components/{name}";
	request: {
		body?: never;
		contentType?: never;
		params: {
			name: string;
		};
		query?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		/** @description The component */
		status: 200;
		contentType: "application/json";
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		};
	} | {
		/** @description Missing or wrong token */
		status: 401;
		contentType: "application/json";
		body: {
			type: "error";
			message: string;
		};
	} | {
		/** @description No such component */
		status: 404;
		contentType: "application/json";
		body: {
			type: "error";
			message: string;
		};
	};
};
/** @summary List items */
export type GetShopItems = {
	method: "get";
	path: "/shop/items";
	request: {
		body?: never;
		contentType?: never;
		params?: never;
		query?: {
			limit?: number;
		};
		headers?: never;
		cookies?: never;
	};
	response: {
		/** @description The items */
		status: 200;
		contentType: "application/json";
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		}[];
	};
};
export type ShopItems = {
	/** @summary List items */
	get: {
		/** @description The items */
		status: 200;
		contentType: "application/json";
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		}[];
	};
	/** @deprecated */
	post: {
		/** @description Created */
		status: 201;
		contentType: null;
		body?: never;
	};
};
export type Ok = {
	/** @description The component */
	"get /components/{name}": {
		type: "component";
		/** @description Server-assigned id */
		readonly id?: string;
		component: Record<string, unknown>;
		tags?: string[] | null;
	};
	/** @description The items */
	"get /shop/items": {
		type: "component";
		/** @description Server-assigned id */
		readonly id?: string;
		component: Record<string, unknown>;
		tags?: string[] | null;
	}[];
};
export type PutComponentsByNameBody = {
	"application/json": {
		type: "component";
		/** @description Server-assigned id */
		readonly id?: string;
		component: Record<string, unknown>;
		tags?: string[] | null;
	};
	"application/octet-stream": Blob;
};
export type PostShopItemsBody = {
	"application/json": {
		type: "component";
		/** @description Server-assigned id */
		readonly id?: string;
		component: Record<string, unknown>;
		tags?: string[] | null;
	};
};
/** @description A reusable component */
export type ComponentWrapped = {
	name: "Component";
	value: {
		type: "component";
		/** @description Server-assigned id */
		readonly id?: string;
		component: Record<string, unknown>;
		tags?: string[] | null;
	};
};
export type ErrorWrapped = {
	name: "Error";
	value: {
		type: "error";
		message: string;
	};
};
export type MixedWrapped = {
	name: "Mixed";
	value: Array<string | 1 | -2 | null>;
};

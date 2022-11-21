export type QueryType = 'branch' | 'tag' | 'commit';

export interface Query {
  value: string;
  type: QueryType;
}

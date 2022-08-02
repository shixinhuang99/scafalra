import * as https from 'node:https'
import { agent } from './utils.js'
import type { Query } from './types.js'
import { ScafalraError } from './error.js'

interface RepositoryResult {
  oid: string
  zipballUrl: string
  url: string
}

interface ApiResultByDefaultBranch {
  repository: {
    url: string
    defaultBranchRef: {
      target: {
        oid: string
        zipballUrl: string
      }
    }
  }
}

interface ApiResultByRef {
  repository: {
    url: string
    object: {
      oid: string
      zipballUrl: string
    }
  }
}

interface ApiParams {
  owner: string
  name: string
}

interface ApiParamsWithQuery extends ApiParams {
  query: Query
}

export class GitHubGraphQLApi {
  private readonly endpoint = 'https://api.github.com/graphql'

  private token = ''

  setToken(token: string) {
    this.token = token
  }

  private buildRepositoryParams(name: string, owner: string) {
    return `name: "${name}", owner: "${owner}", followRenames: false`
  }

  private request<T>(query: string) {
    if (!this.token) {
      throw new ScafalraError('GitHub personal access token is not configured.')
    }
    return new Promise<T>((resolve, reject) => {
      const req = https.request(
        this.endpoint,
        {
          agent,
          method: 'POST',
          headers: {
            'Authorization': `bearer ${this.token}`,
            'Content-Type': 'application/json',
            'User-Agent': 'scafalra',
          },
        },
        (res) => {
          res.setEncoding('utf-8')
          let chunks = ''
          res.on('data', (chunk) => {
            chunks += chunk
          })
          res.on('end', () => {
            const result = JSON.parse(chunks)
            if (result.errors) {
              return reject(new Error(result.errors[0].message))
            }
            if (res.statusCode === 200) {
              return resolve(result.data)
            }
            return reject(new Error(result.message))
          })
          res.on('error', reject)
        }
      )
      req.write(JSON.stringify({ query }))
      req.end()
    })
  }

  private async getRepositoryByDefaultBranch(params: ApiParams) {
    const { name, owner } = params
    const data = await this.request<ApiResultByDefaultBranch>(`{
      repository(${this.buildRepositoryParams(name, owner)}) {
        url
        defaultBranchRef {
          target {
            ... on Commit {
              oid
              zipballUrl
            }
          }
        }
      }
    }`)
    const {
      defaultBranchRef: { target },
      url,
    } = data.repository
    return { ...target, url }
  }

  private async getRepositoryByBranchOrTag(params: ApiParamsWithQuery) {
    const { name, owner, query } = params
    const prefix = query.type === 'branch' ? 'refs/heads' : 'refs/tags'
    const data = await this.request<ApiResultByRef>(`{
      repository(${this.buildRepositoryParams(name, owner)}) {
        url
        object(expression: "${prefix}/${query.value}") {
          ... on Commit {
            oid
            zipballUrl
          }
        }
      }
    }`)
    const { object, url } = data.repository
    return { ...object, url }
  }

  private async getRepositoryByCommit(params: ApiParamsWithQuery) {
    const { name, owner, query } = params
    const data = await this.request<ApiResultByRef>(`{
      repository(${this.buildRepositoryParams(name, owner)}) {
        url
        object(oid: "${query.value}") {
          ... on Commit {
            oid
            zipballUrl
          }
        }
      }
    }`)
    const { object, url } = data.repository
    return { ...object, url }
  }

  get(repo: { owner: string; name: string; query?: Query }): Promise<RepositoryResult> {
    const { owner, name, query } = repo
    if (query?.type === 'branch' || query?.type === 'tag') {
      return this.getRepositoryByBranchOrTag({ owner, name, query })
    }
    if (query?.type === 'commit') {
      return this.getRepositoryByCommit({ owner, name, query })
    }
    return this.getRepositoryByDefaultBranch({ owner, name })
  }
}

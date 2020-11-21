import "./App.scss";
import React from 'react';
import BN from 'bn.js';
import * as nearAPI from 'near-api-js'
import InputNumber from 'react-input-number';
import Timer from 'react-compound-timer';

const IsMainnet = true;
const TestNearConfig = {
  networkId: 'testnet',
  nodeUrl: 'https://rpc.testnet.near.org',
  bananaContractName: 'dev-1604708520705-2360364',
  contractName: 'dev-1605908677227-6741841',
  walletUrl: 'https://wallet.testnet.near.org',
};
const MainNearConfig = {
  networkId: 'mainnet',
  nodeUrl: 'https://rpc.mainnet.near.org',
  bananaContractName: 'berryclub.ek.near',
  contractName: 'farm.berryclub.ek.near',
  walletUrl: 'https://wallet.near.org',
};
const NearConfig = IsMainnet ? MainNearConfig : TestNearConfig;

const Avocado = <span role="img" aria-label="avocado" className="berry">🥑</span>;
const Banana = <span role="img" aria-label="banana" className="berry">🍌</span>;
const Cucumber = <span role="img" aria-label="cucumber" className="berry">🥒</span>;
const Near = <span role="img" aria-label="near" className="berry">Ⓝ</span>;

const Berry = {
  Avocado: 'Avocado',
  Banana: 'Banana',
};

class App extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      connected: false,
      signedIn: false,
      accountId: null,
      account: null,
      claiming: false,
      bananaNum: 1,
    };
    this._balanceRefreshTimer = null;

    this._initNear().then(() => {
      this.setState({
        connected: true,
        signedIn: !!this._accountId,
        accountId: this._accountId,
      });
    });
  }

  componentDidMount() {

  }

  parseBananaAccount(account, accountId) {
    if (!account) {
      account = {
        accountId,
        accountIndex: -1,
        avocadoBalance: 25.0,
        bananaBalance: 0.0,
        bananaBalanceBN: new BN(0),
        numPixels: 0,
        farmingPreference: Berry.Avocado,
      }
    } else {
      account = {
        accountId: account.account_id,
        accountIndex: account.account_index,
        avocadoBalance: parseFloat(account.avocado_balance) / this._pixelCost,
        bananaBalance: parseFloat(account.banana_balance) / this._pixelCost,
        bananaBalanceBN: new BN(account.banana_balance),
        numPixels: account.num_pixels,
        farmingPreference: account.farming_preference,
      }
    }
    account.startTime = new Date().getTime();
    account.avocadoPixels = (account.farmingPreference === Berry.Avocado) ? (account.numPixels + 1) : 0;
    account.bananaPixels = (account.farmingPreference === Berry.Banana) ? (account.numPixels) : 0;
    account.avocadoRewardPerMs = account.avocadoPixels / (24 * 60 * 60 * 1000);
    account.bananaRewardPerMs = account.bananaPixels / (24 * 60 * 60 * 1000);
    account.bananaRewardPerMsBN = account.bananaBalanceBN / new BN(24 * 60 * 60 * 1000);
    return account;
  }

  async getAccount(accountId, stats) {
    const account = this.parseBananaAccount(
      await this._bananaContract.get_account({account_id: accountId}),
      accountId
    );
    let cucumberAccount = await this._contract.get_account({account_id: accountId});
    if (!cucumberAccount) {
      Object.assign(account, {
        nearBalanceBn: new BN(0),
        cucumberBalanceBn: new BN(0),
        nearClaimed: 0,
      });
    } else {
      Object.assign(account, {
        nearBalanceBn: new BN(cucumberAccount.near_balance),
        cucumberBalanceBn: new BN(cucumberAccount.cucumber_balance),
        nearClaimed: parseFloat(cucumberAccount.near_claimed) / Math.pow(10, 24),
      });
    }
    account.nearBalance = parseFloat(account.nearBalanceBn.toString()) / Math.pow(10, 24);
    account.cucumberBalance = parseFloat(account.cucumberBalanceBn.toString()) / this._pixelCost;
    account.percent = account.cucumberBalance * 100 / stats.totalSupply;
    return account;
  }

  async refreshStats() {
    const currentTime = new Date().getTime();
    const lastReward = parseFloat(await this._bananaContract.get_last_reward_timestamp()) / 1e6;
    const rawStats = await this._contract.get_stats();
    const stats = {
      totalSupplyBn: new BN(rawStats.total_cucumber_balance),
      totalSupply: parseFloat(rawStats.total_cucumber_balance) / this._pixelCost,
      totalNearClaimed: parseFloat(rawStats.total_near_claimed) / Math.pow(10, 24),
      totalNearRewarded: parseFloat(rawStats.total_near_received) / Math.pow(10, 24),
      timeUntilRewards: 1606019138008.904777 - currentTime,
      timeFromLastRewards: currentTime - lastReward,
    };
    this.setState({
      stats,
    })
  }

  async refreshAccountStats() {
    await this.refreshStats();
    let account = await this.getAccount(this._accountId, this.state.stats);

    if (this._balanceRefreshTimer) {
      clearInterval(this._balanceRefreshTimer);
      this._balanceRefreshTimer = null;
    }

    this.setState({
      account,
    });

    this._balanceRefreshTimer = setInterval(() => {
      const t = new Date().getTime() - account.startTime;
      this.setState({
        account: Object.assign({}, account, {
          avocadoBalance: account.avocadoBalance + t * account.avocadoRewardPerMs,
          bananaBalance: account.bananaBalance + t * account.bananaRewardPerMs,
        }),
      });
    }, 100);
  }

  async _initNear() {
    const keyStore = new nearAPI.keyStores.BrowserLocalStorageKeyStore();
    const near = await nearAPI.connect(Object.assign({ deps: { keyStore } }, NearConfig));
    this._keyStore = keyStore;
    this._near = near;

    this._walletConnection = new nearAPI.WalletConnection(near, NearConfig.contractName);
    this._accountId = this._walletConnection.getAccountId();

    this._account = this._walletConnection.account();
    this._bananaContract = new nearAPI.Contract(this._account, NearConfig.bananaContractName, {
      viewMethods: ['get_account', 'get_last_reward_timestamp', 'get_account_by_index', 'get_lines', 'get_line_versions', 'get_pixel_cost', 'get_account_balance', 'get_account_num_pixels', 'get_account_id_by_index'],
      changeMethods: ['transfer_with_vault',],
    });
    this._contract = new nearAPI.Contract(this._account, NearConfig.contractName, {
      viewMethods: ['account_exists', 'get_account', 'get_stats', 'get_near_balance', 'get_total_near_claimed', 'get_total_near_received', 'get_balance', 'get_total_supply'],
      changeMethods: ['claim_near', 'transfer_raw'],
    });
    this._pixelCostBN = new BN(await this._bananaContract.get_pixel_cost());
    this._pixelCost = parseFloat(this._pixelCostBN.toString());
    if (this._accountId) {
      await this.refreshAccountStats();
    } else {
      await this.refreshStats();
    }
  }

  async requestSignIn() {
    const appTitle = 'Berry Farm';
    await this._walletConnection.requestSignIn(
        NearConfig.contractName,
        appTitle
    )
  }

  async logOut() {
    this._walletConnection.signOut();
    this._accountId = null;
    this.setState({
      signedIn: !!this._accountId,
      accountId: this._accountId,
    })
  }

  async stakeBananas(bananas) {
    await this.refreshAccountStats();
    if (bananas) {
      bananas = new BN(Math.trunc(bananas * 100000)) * this._pixelCostBN / new BN(100000);
    } else {
      bananas = this.state.account.bananaBalanceBN;
    }
    await this._bananaContract.transfer_with_vault({
      receiver_id: NearConfig.contractName,
      amount: bananas.toFixed(0).toString(),
      payload: '"DepositAndStake"',
    }, new BN("50000000000000"), new BN("1"))
  }

  async claimNear() {
    this.setState({
      claiming: true
    });
    await this._contract.claim_near();
    await this.refreshAccountStats();
    this.setState({
      claiming: false
    });
  }

  render() {
    const account = this.state.account;
    const fraction = 3;
    const content = !this.state.connected ? (
        <div>Connecting... <span className="spinner-grow spinner-grow-sm" role="status" aria-hidden="true"></span></div>
    ) : (this.state.signedIn ? (
        <div>
          <div className="float-right">
            <button
              className="btn btn-outline-secondary"
              onClick={() => this.logOut()}>Log out ({this.state.accountId})</button>
          </div>
          <div>
            { account ? (
              <div className="lines">
                <h3>Your Balances</h3>
                  <div className="balances">
                    {Avocado}{' '}{account.avocadoBalance.toFixed(fraction)}
                    {(account.avocadoPixels > 0) ? (
                      <span>
                        {' (+'}{account.avocadoPixels}{Avocado}{'/day)'}
                      </span>
                    ) : ""}
                  </div>
                  <div className="balances">
                    {Banana}{' '}{account.bananaBalance.toFixed(fraction)}
                    {(account.bananaPixels > 0) ? (
                      <span>
                        {' (+'}{account.bananaPixels}{Banana}{'/day)'}
                      </span>
                    ) : ""}
                  </div>
                  <div>
                    <div>
                      <span className="balances label-for-swap">{Banana}</span>
                      <InputNumber
                        className="balances swap-input"
                        min={0.001}
                        max={this.state.account.bananaBalance}
                        value={this.state.bananaNum}
                        onChange={(bananaNum) => this.setState({bananaNum})}
                        enableMobileNumericKeyboard
                      />
                      <button
                        className={"btn-max balances"}
                        disabled={account.bananaBalance === 0}
                        onClick={() => this.setState({bananaNum: account.bananaBalance.toFixed(3)})}
                      >
                        MAX
                      </button>

                    <Swap
                      account={this.state.account}
                      stakeBananas={(b) => this.stakeBananas(b)}
                      amount={this.state.bananaNum}
                    />
                    </div>
                  </div>
                  <div className="balances">
                    {Cucumber}{' '}{account.cucumberBalance.toFixed(fraction)}{' ('}{account.percent.toFixed(fraction)}{'% share)'}
                  </div>
                  <div>
                    <button
                      className={"btn btn-success" + ((account.nearBalance > 0) ? " btn-large" : " hidden")}
                      disabled={this.state.claiming}
                      onClick={() => this.claimNear()}
                    >
                      Claim {account.nearBalance.toFixed(fraction)} {Near}
                    </button>
                  </div>
                  <div className="balances">
                    Earned {Near}{(account.nearClaimed + account.nearBalance).toFixed(fraction)}
                  </div>
              </div>
            ) : ""}
            </div>
        </div>
    ) : (
        <div style={{marginBottom: "10px"}}>
          <button
              className="btn btn-primary"
              onClick={() => this.requestSignIn()}>Log in with NEAR Wallet</button>
        </div>
    ));
    const stats = this.state.stats ? (
        <div>
          {(this.state.stats.timeUntilRewards > 0) ? (
            <div>
              <h3>Countdown until farming begins</h3>
              <div className="timer">
                <Timer
                  initialTime={this.state.stats.timeUntilRewards}
                  direction="backward"
                  timeToUpdate={100}
                  lastUnit="h"
                >
                  {() => (
                    <React.Fragment>
                      <Timer.Hours />:
                      <Timer.Minutes formatValue={v => `${v}`.padStart(2, '0')}/>:
                      <Timer.Seconds formatValue={v => `${v}`.padStart(2, '0')} />.
                      <Timer.Milliseconds formatValue={v => `${v}`.padStart(3, '0')} />
                    </React.Fragment>
                  )}
                </Timer>
              </div>
            </div>
            ) : (
              <div>
                <h3>Time from last reward</h3>
                <div className="timer small">
                  <Timer
                    initialTime={this.state.stats.timeFromLastRewards}
                    direction="forward"
                    timeToUpdate={100}
                    lastUnit="h"
                  >
                    {() => (
                      <React.Fragment>
                        <Timer.Hours />:
                        <Timer.Minutes formatValue={v => `${v}`.padStart(2, '0')}/>:
                        <Timer.Seconds formatValue={v => `${v}`.padStart(2, '0')} />.
                        <Timer.Milliseconds formatValue={v => `${v}`.padStart(3, '0')} />
                      </React.Fragment>
                    )}
                  </Timer>
                </div>
                Use {Avocado} to draw on berry club to trigger {Near} rewards distribution.
              </div>
          )}
          <h3>Global Farming Stats</h3>
          <div className="lines">
            <div>
              <span className="label">Total {Cucumber} Supplied</span>
              <span className="balances">{this.state.stats.totalSupply.toFixed(3)}</span>
            </div>
            <div>
              <span className="label">Total {Near} Rewarded</span>
              <span className="balances">{this.state.stats.totalNearRewarded.toFixed(3)}</span>
            </div>
            <div>
              <span className="label">Total {Near} Claimed</span>
              <span className="balances">{this.state.stats.totalNearClaimed.toFixed(3)}</span>
            </div>

          </div>
        </div>
      ) : "";
    return (
        <div className="container">
          <div className="row">
            <div>
              <div>
              <h2>Berry Farm {Cucumber}</h2>
              <a
                className="btn btn-outline-none"
                href="https://berryclub.io">{Avocado} Berry Club {Banana}
              </a>
              </div>
              <div className="call-to-action">
                Swap {Banana} to stake {Cucumber} to farm {Near}
              </div>
              {content}
              {stats}
              <div>
              </div>
            </div>
          </div>
        </div>
    );
  }
}

const Swap = (props) => {
  return (
    <button
      className={"btn btn-large" + (props.amount === 0 ? " btn-success" : "")}
      disabled={props.account.bananaBalance < props.amount}
      onClick={() => props.stakeBananas(props.amount)}
    >
      Swap <span className="font-weight-bold">{props.amount || ""}{props.amount ? "" : "ALL"}{Banana}</span> to <span className="font-weight-bold">{Cucumber}</span>
    </button>
  );
}

/*
const Account = (props) => {
  const accountId = props.accountId;
  const shortAccountId = (accountId.length > 6 + 6 + 3) ?
    (accountId.slice(0, 6) + '...' + accountId.slice(-6)) :
    accountId;
  return <a className="account"
            href={`https://wayback.berryclub.io/${accountId}`}>{shortAccountId}</a>
}
*/
export default App;
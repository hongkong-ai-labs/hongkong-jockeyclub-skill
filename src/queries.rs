//! Exact GraphQL operations served by bet.hkjc.com.
//!
//! HKJC accepts only whitelisted query documents, so these strings are kept
//! verbatim. Do not reformat them.

pub const RACE_MEETINGS: &str = r#####"fragment raceFragment on Race {
    id
    no
    status
    raceName_en
    raceName_ch
    postTime
    country_en
    country_ch
    distance
    wageringFieldSize
    go_en
    go_ch
    ratingType
    raceTrack {
      description_en
      description_ch
    }
    raceCourse {
      description_en
      description_ch
      displayCode
    }
    claCode
    raceClass_en
    raceClass_ch
    judgeSigns {
      value_en
    }
  }fragment racingBlockFragment on RaceMeeting {
    jpEsts: pmPools(oddsTypes: [WIN,PLA,TCE,TRI,FF,QTT,DT,TT,SixUP], filters: ["jackpot", "estimatedDividend"]) {
      leg {
        number
        races
      }
      oddsType
      jackpot
      estimatedDividend
      mergedPoolId
    }
    poolInvs: pmPools(oddsTypes: [WIN,PLA,QIN,QPL,CWA,CWB,CWC,IWN,FCT,TCE,TRI,FF,QTT,DBL,TBL,DT,TT,SixUP]) {
      id
      leg {
        races
      }
    }
    penetrometerReadings(filters:["first"]) {
      reading
      readingTime
    }
    hammerReadings(filters:["first"]) {
      reading
      readingTime
    }
    changeHistories(filters:["top3"]) {
      type
      time
      raceNo
      runnerNo
      horseName_ch
      horseName_en
      jockeyName_ch
      jockeyName_en
      scratchHorseName_ch
      scratchHorseName_en
      handicapWeight
      scrResvIndicator
    }
  }
        query raceMeetings($date: String, $venueCode: String) {
            timeOffset {
              rc
            }
            activeMeetings : raceMeetings
            {
              id
              venueCode
              date
              status
              races {
                no
                postTime
                status
                wageringFieldSize
              }
              poolInvs: pmPools(oddsTypes: [WIN,PLA,QIN,QPL,CWA,CWB,CWC,IWN,FCT,TCE,TRI,FF,QTT,DBL,TBL,DT,TT,SixUP]) {
                status
              }
            }
            raceMeetings(date: $date, venueCode: $venueCode)
            {
              id
              status
              venueCode
              date
              totalNumberOfRace
              currentNumberOfRace
              dateOfWeek
              meetingType
              totalInvestment
              country {
                code
                namech
                nameen
                seq
              }
              races {
                ...raceFragment
                runners {
                    id
                    no
                    standbyNo
                    status
                    name_ch
                    name_en
                    horse {
                      id
                      code
                    }
                    color
                    barrierDrawNumber
                    handicapWeight
                    currentWeight
                    currentRating
                    internationalRating
                    gearInfo
                    racingColorFileName
                    allowance
                    trainerPreference
                    last6run
                    saddleClothNo
                    trumpCard
                    priority
                    finalPosition
                    deadHeat
                    winOdds
                    jockey {
                      code
                      name_en
                      name_ch
                    }
                    trainer {
                      code
                      name_en
                      name_ch
                    }
                }
              }
              obSt: pmPools(oddsTypes: [WIN, PLA]) {
                leg {
                  races
                }
                oddsType
                comingleStatus
              }
              poolInvs: pmPools(oddsTypes: [WIN,PLA,QIN,QPL,CWA,CWB,CWC,IWN,FCT,TCE,TRI,FF,QTT,DBL,TBL,DT,TT,SixUP]) {
                id
                leg {
                  number
                  races
                }
                status
                sellStatus
                oddsType
                investment
                mergedPoolId
                lastUpdateTime
              }
              ...racingBlockFragment
              pmPools(oddsTypes: []) {
                id
              }
              jkcInstNo : foPools(oddsTypes: [JKC], filters: ["top"]) {
                instNo
              }
              tncInstNo : foPools(oddsTypes: [TNC], filters: ["top"]) {
                instNo
              }
            }
        }
    "#####;

pub const RESULT_MEETINGS: &str = r#####"fragment racingFoPoolFragment on RacingFoPool {
    instNo
    poolId
    oddsType
    status
    sellStatus
    otherSelNo
    inplayUpTo
    expStartDateTime
    expStopDateTime
    raceStopSellNo
    raceStopSellStatus
    includeRaces
    excludeRaces
    lastUpdateTime
    selections {
      order
      number
      code
      name_en
      name_ch
      scheduleRides
      remainingRides
      points
      lineId
      combId
      combStatus
      openOdds
      prevOdds
      currentOdds
      results {
        raceNo
        points
        point1st
        point2nd
        point3rd
        dhRmk1st
        dhRmk2nd
        dhRmk3rd
        count1st
        count2nd
        count3rd
        count4th
        numerator4th
        denominator4th
      }
    }
    otherSelections {
      order
      code
      name_en
      name_ch
      scheduleRides
      remainingRides
      points
      results {
        raceNo
        points
        point1st
        point2nd
        point3rd
        dhRmk1st
        dhRmk2nd
        dhRmk3rd
        count1st
        count2nd
        count3rd
        count4th
        numerator4th
        denominator4th
      }
    }
  }
  query resultMeetings($date: String, $venueCode: String, $foOddsTypes: [OddsType], $foFilter: [String], $resultOddsType: [OddsType]) {
      raceMeetings(date: $date, venueCode: $venueCode)
      {
        id
        resPools: pmPools(oddsTypes: $resultOddsType) {
          leg {
            number
            races
          }
          status
          oddsType
          name_en
          name_ch
          lastUpdateTime
          dividends (officialOnly: true) {
            winComb
            type
            div
            seq
            status
            guarantee
            partial
            partialUnit
          }
          cWinSelections {
            composite
            name_ch
            name_en
            starters
          }
        }
        foPools(oddsTypes: $foOddsTypes, filters: $foFilter) {
          ...racingFoPoolFragment
        }
      }
  }
"#####;

pub const RACE_ODDS: &str = r#####"
      query sp4e_racing_pmPool($date: String, $venueCode: String, $oddsTypes: [OddsType], $raceNo: Int) {
          raceMeetings(date: $date, venueCode: $venueCode)
          {
            pmPools(oddsTypes: $oddsTypes, raceNo: $raceNo) {
              id
              status
              sellStatus
              oddsType
              lastUpdateTime
              oddsLastUpdateTime
              guarantee
              minTicketCost
              name_en
              name_ch
              leg {
                number
                races
              }
              cWinSelections {
                composite
                name_ch
                name_en
                starters
              }
              oddsNodes {
                combString
                oddsValue
                hotFavourite
                oddsDropValue
                bankerOdds {
                  combString
                  oddsValue
                }
              }
            }
          }
      }
  "#####;

pub const MARK_SIX_DRAWS: &str = r#####"fragment lotteryDrawsFragment on LotteryDraw {
    id
    year
    no
    openDate
    closeDate
    drawDate
    status
    snowballCode
    snowballName_en
    snowballName_ch
    lotteryPool {
      sell
      status
      totalInvestment
      jackpot
      unitBet
      estimatedPrize
      derivedFirstPrizeDiv
      lotteryPrizes {
        type
        winningUnit
        dividend
      }
    }
    drawResult {
      drawnNo
      xDrawnNo
    }
  }
        query marksixResult($lastNDraw: Int, $startDate: String, $endDate: String, $drawType: LotteryDrawType) {
            lotteryDraws(lastNDraw: $lastNDraw, startDate: $startDate, endDate: $endDate, drawType: $drawType) {
              ...lotteryDrawsFragment
            }
        }
    "#####;
